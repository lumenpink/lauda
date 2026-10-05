export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // Servir o APK diretamente do Cloudflare R2 com streaming
    if (url.pathname === "/lauda-latest.apk") {
      const object = await env.LAUDA_BUCKET.get("lauda-latest.apk");
      if (!object) {
        return new Response("APK not found in R2 bucket", { status: 404 });
      }

      const headers = new Headers();
      object.writeHttpMetadata(headers);
      headers.set("etag", object.httpEtag);
      headers.set("content-type", "application/vnd.android.package-archive");
      headers.set("content-disposition", 'attachment; filename="lauda-latest.apk"');

      return new Response(object.body, { headers });
    }

    // Todas as outras rotas (landing page, version.json, imagens) são servidas via Assets estáticos
    return env.ASSETS.fetch(request);
  },
};
