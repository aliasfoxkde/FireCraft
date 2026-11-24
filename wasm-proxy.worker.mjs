export default { async fetch(request) {
  const url = new URL(request.url);
  if (url.pathname.endsWith("_bg.wasm")) {
    const wasmUrl = "https://github.com/aliasfoxkde/FireCraft/releases/download/v0.2.6/firecraft-web-a259c7543f160268_bg.wasm";
    const response = await fetch(wasmUrl);
    const headers = new Headers(response.headers);
    headers.set("Access-Control-Allow-Origin", "*");
    headers.set("Access-Control-Allow-Methods", "GET");
    headers.set("Access-Control-Allow-Headers", "*");
    headers.set("Cache-Control", "public, max-age=31536000");
    headers.set("Content-Type", "application/wasm");
    return new Response(response.body, { status: response.status, headers });
  }
  return new Response("Not found", { status: 404 });
}};
