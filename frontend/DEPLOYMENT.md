# Deployment Guide

This guide covers deploying the Blend Vault frontend to production.

## Prerequisites

- Node.js 18+
- npm or yarn
- A hosting platform account (Vercel, Netlify, or similar)

## Environment Setup

No environment variables are required. All configuration is hardcoded for security:

- **Vault Contract**: `CCZWCNTCTHO3FE6YCYX6YYWFR3B3BEVICD42RZZFMWSPDEIFPQYW4IHA`
- **USDC Contract**: `CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75`
- **Network**: Stellar Mainnet

## Build for Production

```bash
# Install dependencies
npm install

# Build the application
npm run build
```

This creates an optimized production build in the `dist/` directory.

## Deployment Options

### Option 1: Vercel (Recommended)

1. Install Vercel CLI:
```bash
npm install -g vercel
```

2. Deploy:
```bash
vercel --prod
```

3. Or connect your GitHub repository to Vercel for automatic deployments.

**Vercel Configuration** (`vercel.json`):
```json
{
  "buildCommand": "npm run build",
  "outputDirectory": "dist",
  "framework": "vite"
}
```

### Option 2: Netlify

1. Install Netlify CLI:
```bash
npm install -g netlify-cli
```

2. Deploy:
```bash
netlify deploy --prod --dir=dist
```

**Netlify Configuration** (`netlify.toml`):
```toml
[build]
  command = "npm run build"
  publish = "dist"

[[redirects]]
  from = "/*"
  to = "/index.html"
  status = 200
```

### Option 3: Static Hosting (AWS S3, Cloudflare Pages, etc.)

1. Build the application:
```bash
npm run build
```

2. Upload the contents of the `dist/` directory to your hosting provider.

3. Configure your hosting to serve `index.html` for all routes (SPA fallback).

## Post-Deployment Checklist

- [ ] Test wallet connection with multiple wallet providers
- [ ] Verify USDC balance displays correctly
- [ ] Test deposit flow (approve + deposit)
- [ ] Test withdrawal flow
- [ ] Verify transaction links open correctly
- [ ] Test on mobile devices
- [ ] Check console for errors
- [ ] Verify all contract addresses are correct

## Performance Optimization

The production build includes:

- Code splitting for optimal loading
- Asset minification and compression
- Tree shaking to remove unused code
- Optimized images and SVGs

### Additional Optimizations

1. **Enable gzip/brotli compression** on your hosting platform

2. **Configure CDN** for static assets

3. **Set cache headers**:
```
# For static assets (fonts, images, etc.)
Cache-Control: public, max-age=31536000, immutable

# For index.html
Cache-Control: no-cache
```

## Monitoring

### Error Tracking

Consider integrating error tracking:

```typescript
// Add to src/main.tsx
window.addEventListener('error', (event) => {
  // Send to error tracking service
  console.error('Global error:', event.error);
});
```

### Analytics

Add analytics to track user interactions:

```typescript
// Track wallet connections
StellarWalletsKit.on(KitEventType.STATE_UPDATED, (event) => {
  if (event.payload.address) {
    // analytics.track('wallet_connected');
  }
});
```

## Security Considerations

1. **Contract Addresses**: All contract addresses are hardcoded in `src/utils/stellar.ts`
2. **Network**: Locked to Stellar Mainnet
3. **No API Keys**: Application doesn't require any API keys
4. **CSP Headers**: Consider adding Content Security Policy headers

Example CSP header:
```
Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self' https://*.stellar.org https://*.stellar.gateway.fm;
```

## Custom Domain

1. **Add Custom Domain** in your hosting platform's dashboard

2. **Configure DNS**:
   - For Vercel: Add CNAME record pointing to `cname.vercel-dns.com`
   - For Netlify: Add CNAME record pointing to your Netlify subdomain

3. **Enable HTTPS**: Most platforms provide free SSL certificates automatically

## Troubleshooting

### Build Fails

- Clear node_modules and reinstall: `rm -rf node_modules && npm install`
- Check Node.js version: `node --version` (should be 18+)
- Verify all dependencies are compatible

### Application Errors in Production

- Check browser console for errors
- Verify RPC endpoints are accessible from production
- Test with different wallet providers
- Check that contract IDs are correct

### Slow Loading

- Enable compression on hosting platform
- Verify CDN is configured correctly
- Check bundle size: `npm run build` shows size information
- Consider code splitting for large dependencies

## Maintenance

### Updating Dependencies

```bash
# Check for updates
npm outdated

# Update all dependencies
npm update

# Test thoroughly after updates
npm run build
npm run preview
```

### Monitoring Contract Changes

If the vault contract is upgraded:

1. Update `VAULT_CONTRACT_ID` in `src/utils/stellar.ts`
2. Review contract interface for changes
3. Update contract bindings in `src/contracts/vault.ts` if needed
4. Test all functionality
5. Rebuild and redeploy

## Rollback Plan

1. Keep previous deployment artifacts
2. Most hosting platforms support instant rollback
3. For manual rollback, deploy previous `dist/` directory

## Support

If you encounter deployment issues:

1. Check the [Vite Deployment Docs](https://vitejs.dev/guide/static-deploy.html)
2. Review hosting platform documentation
3. Check browser console for specific errors
4. Verify contract addresses on Stellar Expert
