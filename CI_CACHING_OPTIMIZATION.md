# CI Caching Optimization

## Problem
The GitHub Actions CI was taking 5+ minutes just to install software, primarily due to downloading and building Nix packages from scratch on every run.

## Solution
Implemented a comprehensive caching strategy using three complementary approaches:

### 1. Magic Nix Cache (Primary)
- **Action**: `DeterminateSystems/magic-nix-cache-action@v2`
- **Purpose**: Automatically caches Nix store paths using GitHub's caching infrastructure
- **Benefits**: 
  - No configuration required
  - Works out of the box
  - Handles rate limiting automatically
  - Caches newly created store paths during workflow runs

### 2. Cachix Binary Cache (Secondary)
- **Action**: `cachix/cachix-action@v14`
- **Purpose**: Uses pre-built binary packages from Cachix cache
- **Configuration**:
  ```yaml
  - name: Setup Cachix
    uses: cachix/cachix-action@v14
    with:
      name: devenv
      authToken: '${{ secrets.CACHIX_AUTH_TOKEN }}'
  ```
- **Benefits**:
  - Shares pre-built packages across different environments
  - Reduces build time for common dependencies
  - Works across different CI systems

### 3. Nix Store Caching (Tertiary)
- **Action**: `nix-community/cache-nix-action@v6`
- **Purpose**: Caches the entire Nix store between workflow runs
- **Configuration**:
  ```yaml
  - name: Cache Nix Store
    uses: nix-community/cache-nix-action@v6
    with:
      primary-key: ${{ runner.os }}-nix-${{ hashFiles('**/devenv.lock') }}
      restore-prefixes-first-match: |
        ${{ runner.os }}-nix-
      paths: /nix/store
  ```
- **Benefits**:
  - Caches local Nix store
  - Restores previously built packages
  - Uses devenv.lock as cache key for precise invalidation

## Expected Performance Improvements

### Before Optimization
- **Installation time**: 5+ minutes
- **Total CI time**: 8-12 minutes per job
- **Cache hit rate**: 0% (no caching)

### After Optimization
- **Installation time**: 30-60 seconds (first run), 10-30 seconds (cached runs)
- **Total CI time**: 3-6 minutes per job
- **Cache hit rate**: 80-95% for subsequent runs

## Setup Requirements

### Required Secrets
Add these secrets to your GitHub repository settings:

1. **CACHIX_AUTH_TOKEN**: Your Cachix authentication token
   - Get from: https://cachix.org/
   - Create a new cache named "devenv" or use an existing one

### Optional Secrets
2. **CACHIX_SIGNING_KEY**: Your Cachix signing key (if using custom cache)

## Cache Invalidation Strategy

The cache is invalidated when:
- `devenv.lock` file changes (indicates dependency changes)
- Operating system changes (different runners)
- Manual cache clearing

## Monitoring Cache Performance

### Check Cache Hit Rate
Look for these indicators in CI logs:
- "Cache restored from key: ..." (cache hit)
- "Cache not found for input keys: ..." (cache miss)
- "Post job cleanup" (cache saved)

### Optimize Further
If cache hit rate is low:
1. Check if `devenv.lock` is being committed to repository
2. Verify Cachix cache is properly configured
3. Consider using more specific cache keys

## Files Modified

- `.github/workflows/ci.yml` - Added caching to all 5 jobs
- `.github/workflows/publish.yml` - Added caching to both jobs
- `CI_CACHING_OPTIMIZATION.md` - This documentation

## Cost Impact

### GitHub Actions
- **Cache storage**: ~500MB-2GB per repository (free tier: 10GB)
- **Bandwidth**: Reduced due to cache hits
- **Compute time**: Significantly reduced

### Cachix
- **Free tier**: 5GB storage, 100GB bandwidth/month
- **Paid tiers**: Available for larger projects

## Troubleshooting

### Common Issues

1. **Cache not working**
   - Verify `CACHIX_AUTH_TOKEN` secret is set
   - Check if Cachix cache exists and is accessible
   - Ensure `devenv.lock` is committed to repository

2. **Slow first run**
   - First run will always be slower (no cache)
   - Subsequent runs should be much faster

3. **Cache size limits**
   - GitHub Actions cache: 10GB free tier
   - Cachix free tier: 5GB storage
   - Consider upgrading if hitting limits

### Debug Commands
```bash
# Check cache status
nix path-info --all

# List cached paths
nix-store --query --requisites /nix/store

# Check Cachix cache
cachix list
```

## Future Optimizations

1. **Custom Cachix cache**: Create project-specific cache for better performance
2. **Docker layer caching**: If using Docker, implement layer caching
3. **Parallel jobs**: Run independent jobs in parallel
4. **Selective caching**: Cache only frequently used packages

## References

- [Magic Nix Cache Action](https://github.com/DeterminateSystems/magic-nix-cache-action)
- [Cachix Action](https://github.com/cachix/cachix-action)
- [Cache Nix Action](https://github.com/nix-community/cache-nix-action)
- [Nix.dev CI Guide](https://nix.dev/guides/recipes/continuous-integration-github-actions)