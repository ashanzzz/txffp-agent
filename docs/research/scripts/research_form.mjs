const cdpHost = "192.168.8.11:19223";

async function run() {
  const listRes = await fetch(`http://${cdpHost}/json/list`);
  const pages = await listRes.json();
  const page = pages.find(p => p.url && p.url.includes("txffp.com"));

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

  const evalResult = await send("Runtime.evaluate", {
    expression: `
      (() => {
        const form = document.getElementById('loginForm');
        if (!form) return { error: "No loginForm found" };

        const formElements = Array.from(form.querySelectorAll('*')).map(el => ({
          tag: el.tagName,
          id: el.id,
          className: el.className,
          text: el.innerText ? el.innerText.trim() : "",
          onclick: el.getAttribute('onclick')
        })).filter(e => e.text || e.id || e.onclick);

        return {
          formElements: formElements.slice(0, 30),
          scripts: Array.from(document.querySelectorAll('script')).map(s => s.src).filter(s => s)
        };
      })()
    `,
    returnByValue: true
  });

  console.log("Form elements & scripts:", JSON.stringify(evalResult.result.value, null, 2));
  ws.close();
}

run().catch(console.error);
