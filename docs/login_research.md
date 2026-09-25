# 票根网登录逆向与协议研究报告 (Txffp Login Reverse Engineering)

## 1. 登录架构概览

票根网采用 **OAuth2 单点登录平台 (SSO)** 架构：
- 前台入口：`https://www.txffp.com/pss/app/login/manage`
- 302 重定向至统一认证平台：
  ```text
  https://sso.txffp.com/sso/app/oauth/login?client_id=000031&response_type=code&redirect_uri=https://pss.txffp.com/pss/app/oauth/login&scope=USERINFO&state=state
  ```
- 业务域名：`pss.txffp.com`（发票服务系统）
- 认证域名：`sso.txffp.com`（统一登录平台）

## 2. 登录表单核心字段

表单属性：
- Form ID: `loginForm`
- Action: `https://sso.txffp.com/sso/app/oauth/login`
- Method: `POST`

### 隐藏参数 (Hidden Fields)
| 字段名 | 典型取值 | 说明 |
|---|---|---|
| `client_id` | `000031` | 票根业务系统客户端标识 |
| `response_type` | `code` | OAuth2 授权码模式 |
| `redirect_uri` | `https://pss.txffp.com/pss/app/oauth/login` | 授权成功后的回调地址 |
| `scope` | `USERINFO` | 权限范围 |
| `state` | `state` | 防 CSRF 随机状态串 |
| `loginType` | `PASSWORD` / `SMS` | 登录类型（密码登录或短信验证码登录） |
| `captchaVerifyParam` | 字符串 | 阿里云验证码 2.0 验证成功后返回的凭证字串 |
| `sessionId` | 字符串 | SSO 会话标识 |
| `token` | 字符串 | 防重放 Token |

### 用户输入参数
| 字段名 | 控件 ID | 说明 |
|---|---|---|
| `loginName` | `#loginName` | 登录手机号 / 账号名 |
| `passwd` | `#passwd` | 密码（提交前经前端 AES 加密） |
| `validCode` | `#validCode` | 图形验证码（多次失败后触发） |

## 3. 密码加密算法规范

- **算法**: AES-128 / AES-256
- **模式**: ECB (`CryptoJS.mode.ECB`)
- **填充**: PKCS7 (`CryptoJS.pad.Pkcs7`)
- **密钥**: `#xy@etcchina.com` (UTF-8 编码)
- **输出格式**: Base64 编码字符串

### 前端源码实现：
```javascript
function aesEncrypt(data) {
    var dataEn = CryptoJS.enc.Utf8.parse(data);
    var keyEn = CryptoJS.enc.Utf8.parse("#xy@etcchina.com");
    var ciphertext = CryptoJS.AES.encrypt(dataEn, keyEn, {
        mode: CryptoJS.mode.ECB,
        padding: CryptoJS.pad.Pkcs7
    }).toString();
    return ciphertext;
}
```

## 4. 人机安全验证机制 (Aliyun Captcha 2.0)

- **服务商**: 阿里云人机验证 2.0 (`AliyunCaptcha.js`)
- **配置**:
  - `region`: `cn`
  - `prefix`: `161bqq`
  - `SceneId`: `1723z3jd`
  - `mode`: `embed`（嵌入式直接显示在 `#sc` 容器内）
- **交互逻辑**:
  1. 页面加载时，`#submitButton` 背景色被强制置灰，并移除提交类 `taiji_ajaxForm`，禁止直接提交。
  2. 用户在远程窗口中点击/滑动完成验证。
  3. 验证通过回调 `success(captchaVerifyParam)`：
     - 将验证凭证填入 `$("#captchaVerifyParam").val(captchaVerifyParam)`
     - 恢复按钮颜色并添加 `taiji_ajaxForm`
  4. 提交登录表单。

## 5. 失败策略与频率控制

1. 密码或验证码错误超过阈值时：
   - 接口 `GET /sso/app/person/failedCount` 返回 `json.valid: true`。
   - 激活显示图形验证码 `#validCodeStr`，从 `/sso/app/validCodeImage?ee={index}` 拉取。
2. 短信快捷登录：
   - 切换至 `loginType=SMS`。
   - 点击 `#getRandomCode` 触发短信发送，携带 300 秒倒计时。

## 6. 成功跳转与会话建立

表单提交后，SSO 服务端返回 JSON：
```json
{
  "rediectUrl": "https://pss.txffp.com/pss/app/oauth/login?code=AUTH_CODE&state=state"
}
```
浏览器加载该 `rediectUrl`，`pss.txffp.com` 校验 `AUTH_CODE` 并下发发票业务系统的 Cookie (`JSESSIONID`, `ROUTEID`, `SERVERID`)，正式完成登录。
