FROM node:20-slim AS builder

WORKDIR /app

COPY web-ui/package*.json ./
RUN if [ -f package-lock.json ]; then npm ci; else npm install; fi

COPY web-ui/ ./

ARG VITE_API_URL=/api
ENV VITE_API_URL=$VITE_API_URL

RUN npx vite build

FROM nginx:1.27-alpine

COPY web-ui/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 8080
