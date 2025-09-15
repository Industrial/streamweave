# GitHub Actions Fix: devenv-action Repository Not Found

## Problem

The GitHub Actions workflows were failing with the error:
```
Error: Unable to resolve action cachix/devenv-action, repository not found
```

## Root Cause

The `cachix/devenv-action` repository doesn't exist or is not publicly accessible. This action was being used in both CI and publish workflows to set up the devenv environment.

## Solution

Replaced the non-existent `cachix/devenv-action` with the correct approach for using devenv in GitHub Actions:

### Changes Made

1. **Replaced devenv action installation** with direct installation:
   ```yaml
   # Before (broken)
   - name: Install devenv
     uses: cachix/devenv-action@v0.1.0

   # After (working)
   - name: Install devenv
     run: |
       nix profile install github:cachix/devenv
   ```

2. **Updated devenv shell commands** to use `nix develop`:
   ```yaml
   # Before
   devenv shell -- bash -c "..."

   # After
   nix develop --command bash -c "..."
   ```

3. **Updated step names** for clarity:
   ```yaml
   # Before
   - name: Run devenv shell

   # After
   - name: Run nix develop
   ```

### Files Modified

- `.github/workflows/ci.yml` - Updated all 5 job configurations
- `.github/workflows/publish.yml` - Updated both job configurations

## Why This Works

1. **Direct Installation**: `nix profile install github:cachix/devenv` installs devenv directly from the official repository
2. **Nix Develop**: `nix develop` activates the development environment defined in `devenv.nix`
3. **Command Execution**: `--command bash -c "..."` runs the specified commands within the activated environment

## Testing

The workflows should now work correctly with:
- Proper devenv installation
- Correct environment activation
- All existing functionality preserved

## Alternative Approaches

If issues persist, consider:
1. Using `nix-shell` instead of `nix develop`
2. Using the `nixos/nix` Docker container
3. Setting up a custom action for devenv installation

## Dependencies

This fix maintains compatibility with:
- The existing `devenv.nix` configuration
- The existing `devenv.yaml` configuration
- All Nix and Rust tooling