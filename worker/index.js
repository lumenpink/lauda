export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // Servir APKs diretamente do Cloudflare R2 com streaming
    if (url.pathname === "/lauda-latest.apk" || url.pathname === "/lauda-nightly.apk") {
      const fileName = url.pathname.slice(1);
      const object = await env.LAUDA_BUCKET.get(fileName);
      if (!object) {
        return new Response(`APK ${fileName} not found in R2 bucket`, { status: 404 });
      }

      const headers = new Headers();
      object.writeHttpMetadata(headers);
      headers.set("etag", object.httpEtag);
      headers.set("content-type", "application/vnd.android.package-archive");
      headers.set("content-disposition", `attachment; filename="${fileName}"`);

      return new Response(object.body, { headers });
    }

    // Todas as outras rotas (landing page, version.json, version-nightly.json) são servidas via Assets estáticos
    return env.ASSETS.fetch(request);
  },
};
