const cdpHost = "192.168.8.11:19223";

async function run() {
  const listRes = await fetch(`http://${cdpHost}/json/list`);
  const pages = await listRes.json();
  const page = pages.find(p => p.url && p.url.includes("txffp.com"));
  if (!page) {
    console.error("No txffp.com page found in browser!");
    return;
  }

  let wsUrl = page.webSocketDebuggerUrl;
  if (!wsUrl.includes(":19223")) {
    wsUrl = wsUrl.replace("ws://192.168.8.11/", `ws://${cdpHost}/`);
  }

  const ws = new WebSocket(wsUrl);
  let messageId = 0;
  const pending = new Map();

  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++messageId;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    if (data.id && pending.has(data.id)) {
      const { resolve, reject } = pending.get(data.id);
      pending.delete(data.id);
      if (data.error) reject(data.error);
      else resolve(data.result);
    }
  };

  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });

  console.log("Navigating to login page...");
  await send("Page.navigate", { url: "https://www.txffp.com/pss/app/login/manage" });

  // Wait 3 seconds for page and scripts to load
  await new Promise(r => setTimeout(r, 3500));

  const evalResult = await send("Runtime.evaluate", {
    expression: `
      (() => {
        const inputs = Array.from(document.querySelectorAll('input')).map(i => ({
          name: i.name,
          id: i.id,
          type: i.type,
          placeholder: i.placeholder,
          className: i.className,
          visible: i.offsetParent !== null
        }));

        const tabs = Array.from(document.querySelectorAll('li, div, span, a'))
          .map(el => el.innerText ? el.innerText.trim() : "")
          .filter(t => t.includes("密码登录") || t.includes("验证码登录") || t.includes("扫码登录") || t.includes("手机登录"));

        const forms = Array.from(document.querySelectorAll('form')).map(f => ({
          id: f.id,
          action: f.action,
          method: f.method
        }));

        const buttons = Array.from(document.querySelectorAll('button, a.btn, input[type="button"], input[type="submit"], .btn'))
          .map(b => ({
            tag: b.tagName,
            id: b.id,
            text: b.innerText ? b.innerText.trim() : (b.value || ""),
            className: b.className
          }));

        return {
          title: document.title,
          url: window.location.href,
          tabs: Array.from(new Set(tabs)),
          inputs,
          forms,
          buttons
        };
      })()
    `,
    returnByValue: true
  });

  console.log("Login page structure:", JSON.stringify(evalResult.result.value, null, 2));

  ws.close();
}

run().catch(console.error);
