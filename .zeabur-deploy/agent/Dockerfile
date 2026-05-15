FROM node:20-slim as builder

WORKDIR /app

COPY agent-service/package*.json ./
RUN npm ci

COPY agent-service/tsconfig.json ./
COPY agent-service/src ./src
RUN npm run build

FROM node:20-slim

WORKDIR /app

COPY --from=builder /app/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/package*.json ./

ENV EXECUTOR_URL=http://executor:8080
ENV PORT=3000
ENV NODE_ENV=production

EXPOSE 3000

CMD ["node", "dist/index.js"]