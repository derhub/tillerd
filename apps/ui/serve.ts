import index from "./build/client/index.html" with { type: "file" };

const root = `${import.meta.dir}/build/client`;
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
