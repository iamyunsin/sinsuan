<p align="center">
    English
    |
    <a href="./README-zh.md">简体中文</a>
</p>

# Sinsuan

Sinsuan is an open source static site statistics tool that supports self-built services or online services.  

It is recommended to use your own service, as it will allow you to better back up your site usage and manage your site statistics.  

## Getting Started  

1. Install dependencies  

```bash
npm install sinsuan
```

2. Use for your project  

* Start statistics  

```ts
import { bootstrap } from 'sinsuan'

// start statistics
bootstrap();
```

* Show statistics data  

```html
page PV: <span data-sinsuan-pv></span>
page UV: <span data-sinsuan-uv></span>
site PV: <span data-sinsuan-site-pv></span>
site UV: <span data-sinsuan-site-uv></span>
```

## Self-hosted  

1. Download the latest release

[Download](https://github.com/iamyunsin/sinsuan/releases)

2. Run the server  

```bash
./sinsuan
```

4. Use for your project  
```ts
import { bootstrap } from 'sinsuan'

// 启动统计
bootstrap({
  serverUrl: 'https://your.host/path/to/count'
});
```

## Contact me  

* email: yunsin@vip.qq.com  
* wechat: iamyunsin  

## Encourage me  

If you find this project helpful, you can support the author in the following ways:

1. WeChat Pay  
![](./images/wechat.jpg)

2. Alipay  
![](./images/alipay.jpg)