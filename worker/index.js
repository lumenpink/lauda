export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // Servir APKs diretamente do Cloudflare R2 com streaming e suporte a HTTP Range (206) / HEAD
    if (url.pathname.endsWith(".apk")) {
      const isNightly = url.pathname.includes("nightly");
      const r2Key = isNightly ? "lauda-nightly.apk" : "lauda-latest.apk";
      const clientFileName = url.pathname.slice(1);

      // Tratamento para requisições HEAD (verificação de tamanho e ETag sem baixar corpo)
      if (request.method === "HEAD") {
        const object = await env.LAUDA_BUCKET.head(r2Key);
        if (!object) {
          return new Response(`APK ${r2Key} not found in R2 bucket`, { status: 404 });
        }

        const headers = new Headers();
        object.writeHttpMetadata(headers);
        headers.set("etag", object.httpEtag);
        headers.set("accept-ranges", "bytes");
        headers.set("content-type", "application/vnd.android.package-archive");
        headers.set("content-length", object.size.toString());
        headers.set("content-disposition", `attachment; filename="${clientFileName}"`);

        return new Response(null, { headers });
      }

      if (request.method !== "GET") {
        return new Response("Method Not Allowed", { status: 405 });
      }

      // Tratamento para requisições GET com suporte a Range (essencial para Obtainium e downloads retomáveis)
      const rangeHeader = request.headers.get("range");
      const object = await env.LAUDA_BUCKET.get(r2Key, {
        range: rangeHeader ? request.headers : undefined,
        onlyIf: request.headers,
      });

      if (!object) {
        return new Response(`APK ${r2Key} not found in R2 bucket`, { status: 404 });
      }

      const headers = new Headers();
      object.writeHttpMetadata(headers);
      headers.set("etag", object.httpEtag);
      headers.set("accept-ranges", "bytes");
      headers.set("content-type", "application/vnd.android.package-archive");
      headers.set("content-disposition", `attachment; filename="${clientFileName}"`);

      // Se for uma requisição com Range atendida parcialmente pelo R2
      if (object.range) {
        headers.set(
          "content-range",
          `bytes ${object.range.offset}-${object.range.offset + object.range.length - 1}/${object.size}`
        );
        headers.set("content-length", object.range.length.toString());
        return new Response(object.body, { status: 206, headers });
      }

      headers.set("content-length", object.size.toString());
      return new Response(object.body, { headers });
    }

    // Roteamento da WebApp SPA (/app e /app/)
    if (url.pathname === "/app" || url.pathname === "/app/") {
      const appIndexUrl = new URL("/app/index.html", request.url);
      return env.ASSETS.fetch(new Request(appIndexUrl, request));
    }

    // Se for rota sob /app/* mas não for asset direto (sem extensão de arquivo), faz fallback para /app/index.html
    if (url.pathname.startsWith("/app/") && !url.pathname.includes(".")) {
      const appIndexUrl = new URL("/app/index.html", request.url);
      return env.ASSETS.fetch(new Request(appIndexUrl, request));
    }

    // Todas as outras rotas (landing page, assets, version.json, version-nightly.json) são servidas via Assets estáticos
    return env.ASSETS.fetch(request);
  },
};
