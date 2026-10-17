export default {
  async fetch(request, env) {
    const url = new URL(request.url);

    // Redirecionamento soberano de APKs para o Cloudflare R2 dedicado (bucket.2lp.in) com zero custo de worker
    if (url.pathname.endsWith(".apk")) {
      const isNightly = url.pathname.includes("nightly");
      const targetApk = isNightly ? "lauda-nightly.apk" : "lauda.apk";
      return Response.redirect(`https://bucket.2lp.in/${targetApk}`, 302);
    }

    // Normalizar /app sem barra para /app/
    if (url.pathname === "/app") {
      return Response.redirect(new URL("/app/", request.url).toString(), 301);
    }

    // Se for sub-rota client-side sob /app/* sem extensão, fazer fallback para /app/
    if (url.pathname.startsWith("/app/") && !url.pathname.includes(".") && url.pathname !== "/app/") {
      const appIndexUrl = new URL("/app/", request.url);
      return env.ASSETS.fetch(new Request(appIndexUrl, request));
    }

    // Todas as outras rotas (landing page, assets, version.json, version-nightly.json) são servidas via Assets estáticos
    return env.ASSETS.fetch(request);
  },
};
