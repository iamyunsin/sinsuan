/**
 * Copyright 2024-present iamyunsin
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
/**
 * 本地存储接口
 */
export type Storage = {
  getItem(key: string): string | null | Promise<string | null>;
  setItem(key: string, value: string): void | Promise<void>;
}

/** 后端返回的数据 */
type SinSuanData = {
  /** 用户/浏览器（访问代理）唯一标识 */
  sin_suan_id: string;
  /** 当前页面访问次数 */
  pv: number;
  /** 当前页面访问用户数 */
  uv: number;
  /** 站点总访问次数 */
  site_pv: number;
  /** 站点总访问用户数 */
  site_uv: number;
};

/** 当收到数据时的处理函数 */
export type OnReceiveDataHandler = (data: Omit<SinSuanData, 'sin_suan_id'>) => void;

/**
 * 配置信息
 */
export type SinSuanConfig = {
  /** 服务器接口地址 */
  serverUrl: string;
  /** 传递ID值的请求头 */
  idHeaderKey: string;
  /** 传递统计地址的请求头，由于需要支持单页形式的hash模式路由的请求和避免no-referer页面服务器无法获取Referer头的情况 */
  countUrlHeaderKey: string;
  /** 用于存储信息的实例，可以由使用方实现，默认实现使用localStorage */
  storage: Storage;
  /** 是否启用hash路由统计模式 */
  hashMode: boolean;
  /** 是否启用history路由统计模式 */
  historyMode: boolean;
  /** 当收到数据时要执行的处理 */
  onReceiveData: OnReceiveDataHandler;
};

function setTextInElement(selector: string, text: string) {
  const elements = document.querySelectorAll(selector);
  elements.forEach(element => {
    element.textContent = text;
  });
}

function defaultOnReceiveDataHandler(data: Omit<SinSuanData, 'sin_suan_id'>) {
  // 为了避免hash路由或history路由跳转时html节点更新不及时，延迟300ms
  setTimeout(() => {
    setTextInElement('*[data-sinsuan-uv]', data.uv.toString());
    setTextInElement('*[data-sinsuan-pv]', data.pv.toString());
    setTextInElement('*[data-sinsuan-site-uv]', data.site_uv.toString());
    setTextInElement('*[data-sinsuan-site-pv]', data.site_pv.toString());
  }, 200);
}

/**
 * 默认配置
 */
const defaultConfig: SinSuanConfig = {
  serverUrl: 'https://sinsuan.yunsin.top/count',
  idHeaderKey: 'X-Sinsuan-Id',
  countUrlHeaderKey: 'X-Sinsuan-Count-Url',
  storage: window.localStorage,
  hashMode: false,
  historyMode: true,
  onReceiveData: defaultOnReceiveDataHandler,
};

/**
 * 初始化心算统计
 * @param options 配置信息
 */
export function bootstrap(config: Partial<SinSuanConfig> = {}): void {
  Object.assign(defaultConfig, config);
  initEventListeners();
  const domReady = () => {
    window.removeEventListener('DOMContentLoaded', domReady);
    addVisit();
  }
  window.addEventListener('DOMContentLoaded', domReady);
}

function proxyHistoryMethod(type: 'pushState' | 'replaceState') {
  const originMethod = history[type];
  return (data: any, unused: string, url?: string | URL | null | undefined) => {
    const rv = originMethod.call(history, data, unused, url);
    // 首先URL有值，才派发事件，否则不派发事件
    if (!!url) {
      const event = new Event(type.toLowerCase(), {
        bubbles: true,
        cancelable: true,
        composed: true,
      });
      (event as any).options = {
        data,
        unused,
        url,
      };
      window.dispatchEvent(event);
    }
    return rv;
  };
}

window.history.pushState = proxyHistoryMethod('pushState');
window.history.replaceState = proxyHistoryMethod('replaceState');

function handleRouteChange() {
  addVisit();
}

/** 初始化hash路由和history路由统计事件 */
function initEventListeners() {
  if(defaultConfig.hashMode) {
    window.addEventListener('hashchange', handleRouteChange);
  }
  if(defaultConfig.historyMode) {
    window.addEventListener('popstate', handleRouteChange);
    window.addEventListener('pushstate', handleRouteChange);
    window.addEventListener('replacestate', handleRouteChange);
  }
}

/**
 * 添加访问记录
 */
async function addVisit() {
  const id = await defaultConfig.storage.getItem('__sin_suan_id__');
  const xhr = new XMLHttpRequest();
  xhr.open('GET', defaultConfig.serverUrl);
  xhr.withCredentials = true;
  xhr.setRequestHeader(defaultConfig.countUrlHeaderKey, location.href);
  if (id) {
    xhr.setRequestHeader(defaultConfig.idHeaderKey, id || '');
  }
  xhr.onload = () => {
    const data = JSON.parse(xhr.responseText);
    defaultConfig.storage.setItem('__sin_suan_id__', data.sin_suan_id);
    defaultConfig.onReceiveData(data);
  }
  xhr.onerror = () => {
    console.error('sinsuan error', xhr.statusText);
  }

  xhr.send(null);
}
