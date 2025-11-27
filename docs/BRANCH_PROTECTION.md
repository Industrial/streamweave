# Branch Protection for Documentation Updates

This document describes the recommended branch protection settings for documentation updates in the StreamWeave repository.

## Overview

To ensure documentation quality and consistency, certain branches should be protected to require reviews and checks before documentation changes can be merged.

## Recommended Settings

### Main Branch Protection

For the `main` branch, enable the following protections:

1. **Require pull request reviews before merging**
   - Required number of reviewers: 1
   - Dismiss stale pull request approvals when new commits are pushed: Yes
   - Require review from Code Owners: Yes (if CODEOWNERS file exists)

2. **Require status checks to pass before merging**
   - Required status checks:
     - `docs` - Documentation build check
     - `lint` - Linting checks
     - `test` - Test suite
   - Require branches to be up to date before merging: Yes

3. **Require conversation resolution before merging**: Yes

4. **Do not allow bypassing the above settings**: Recommended for main branch

### Documentation-Specific Branches

For branches that primarily contain documentation changes (e.g., `docs/*`):

1. **Require pull request reviews before merging**: Yes (1 reviewer)
2. **Require status checks**: Documentation build must pass
3. **Allow force pushes**: No
4. **Allow deletions**: No

## GitHub Settings Location

These settings can be configured in:
- Repository Settings → Branches → Branch protection rules
- Add rule → Branch name pattern (e.g., `main` or `docs/*`)
- Configure the above settings

## CI/CD Integration

The documentation build step in `.github/workflows/ci.yml` will automatically run on pull requests. The branch protection rules ensure that:

1. Documentation must build successfully
2. No documentation warnings are introduced
3. All links in documentation are valid

## Documentation Review Checklist

When reviewing documentation PRs, check:

- [ ] Documentation builds without errors
- [ ] All code examples compile and run
- [ ] Links are valid and working
- [ ] Documentation follows the style guide
- [ ] New features are documented
- [ ] Breaking changes are clearly marked
- [ ] Examples are updated if APIs change

## Exceptions

For minor documentation fixes (typos, formatting), maintainers may:
- Use the "bypass branch protection" option if available
- Merge directly to main for trivial changes
- Use fast-forward merges for documentation-only changes

## Automation

Consider setting up:
- Automated documentation checks via GitHub Actions
- Documentation link checker
- Spell checker for documentation files
- Automatic generation of documentation coverage reports

