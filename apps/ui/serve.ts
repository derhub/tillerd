import index from "./dist/index.html" with { type: "file" };

const root = `${import.meta.dir}/dist`;
const port = Number(process.env.PORT ?? 3001);

Bun.serve({
  port,
  async fetch(req) {
    const url = new URL(req.url);
    const asset = Bun.file(`${root}${url.pathname}`);
    if (url.pathname !== "/" && (await asset.exists())) {
      return new Response(asset);
    }
    return new Response(Bun.file(index));
  },
});
