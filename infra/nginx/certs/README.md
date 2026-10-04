# TLS Certificates Configuration

This directory contains TLS certificates for the Nginx ingress gateway.

Required files for Nginx:
- `fullchain.pem`: Combined server certificate and CA certificate chain
- `privkey.pem`: Private key matching the certificate

## 1. Local Development Setup

For local testing over HTTPS and HTTP/3 QUIC, generate a self-signed certificate:

```bash
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout infra/nginx/certs/privkey.pem \
  -out infra/nginx/certs/fullchain.pem \
  -subj "/CN=localhost"
```

Set appropriate read permissions:

```bash
chmod 600 infra/nginx/certs/privkey.pem
chmod 644 infra/nginx/certs/fullchain.pem
```

## 2. Production Deployment Setup

For production public domains, obtain certificates via Certbot and Let's Encrypt:

1. Request certificates:
   ```bash
   certbot certonly --standalone -d proofbattle.example.com
   ```

2. Symlink or copy certificates into this directory:
   ```bash
   cp /etc/letsencrypt/live/proofbattle.example.com/fullchain.pem infra/nginx/certs/fullchain.pem
   cp /etc/letsencrypt/live/proofbattle.example.com/privkey.pem infra/nginx/certs/privkey.pem
   ```

3. Ensure automated renewal with a cron job or systemd timer:
   ```bash
   certbot renew --post-hook "docker compose -f /path/to/infra/docker-compose.yml exec nginx nginx -s reload"
   ```
