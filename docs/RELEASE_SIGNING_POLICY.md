# Release Signing Policy

This document defines the minimum operational policy for signing Pulse release artifacts.

## Scope

- Signed artifacts:
  - Windows MSI installers
  - Unix tarballs
  - VS Code extension VSIX
- Signature format: `cosign sign-blob` detached `.sig`.

## Key Management

- Keys are managed outside the repository.
- CI obtains signing material only from protected secrets:
  - `COSIGN_PRIVATE_KEY`
  - `COSIGN_PASSWORD`
  - `COSIGN_PUBLIC_KEY` (or derived from private key in CI)
- Keys must be rotated at least every 180 days.
- Emergency rotation window: within 24 hours of compromise suspicion.

## Rotation Procedure

1. Generate a new keypair offline.
2. Update CI secrets with new private/public key material.
3. Trigger a release run with `require_signing=true`.
4. Verify signatures for all artifacts in the workflow output.
5. Revoke old public key in downstream trust stores.

## Publication Requirements

- Release is blocked when `require_signing=true` and key material is missing.
- Checksums (`SHA256SUMS-*.txt`) and signatures (`*.sig`) must be published together.
- Verification step must pass in CI before release approval.

## Verification Command

```bash
cosign verify-blob --key cosign.pub --signature artifact.sig artifact
```

