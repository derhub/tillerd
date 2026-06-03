FROM oven/bun:1 AS build-env
COPY . /app/
WORKDIR /app
RUN bun install --frozen-lockfile
RUN bun run build

FROM oven/bun:1
WORKDIR /app
COPY --from=build-env /app/build/client /app/build/client
COPY --from=build-env /app/serve.ts /app/serve.ts
CMD ["bun", "run", "serve.ts"]
