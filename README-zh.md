<p align="center">
    <a href="./README.md">English</a>
    |
    简体中文
</p>

# 心算(sinsuan)

心算是一个开源的静态站点统计工具，支持自建服务，也可以使用在线服务。    

建议使用自建服务，因为这样您可以更好地备份使用和管理自己的站点统计数据。  

## 快速开始

1. 安装依赖

```bash
npm install sinsuan
```

2. 项目中集成  

* 启动统计  

```ts
import { bootstrap } from 'sinsuan'

// 启动统计
bootstrap();
```

* HTML节点设置  

```html
当前页面PV: <span data-sinsuan-pv></span>
当前页面UV: <span data-sinsuan-uv></span>
站点PV: <span data-sinsuan-site-pv></span>
站点UV: <span data-sinsuan-site-uv></span>
```

## 自建服务  

1. 下载服务启动器  

[点击下载](https://github.com/iamyunsin/sinsuan/releases)

2. 启动服务器  

```bash
./sinsuan
```

3. 客户端集成  

```ts
import { bootstrap } from 'sinsuan'

// 启动统计
bootstrap({
  serverUrl: 'https://your.host/path/to/count'
});
```

## 联系我  

* 邮箱: yunsin@vip.qq.com  
* 微信: iamyunsin  

## 鼓励我

如果觉得这个项目对您有帮助，您可以通过以下方式支持作者：

1. 微信  
![](./images/wechat.jpg) 

2. 支付宝  
![](./images/alipay.jpg)