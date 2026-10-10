---
core.name: AltStore Source
core.desc: The AltStore source JSON that distributes iOS builds to sideloading users.
core.category:
  - core.concept
core.belongs:
  - distribution-and-release
---

`altstore-source.json` at the repository root is an AltStore source file.
AltStore sources are self-hosted JSON documents that describe apps,
their versions, and download locations.
Users add the source URL in AltStore to browse and install listed apps.

The source is publicly accessible at the raw GitHub URL:

```
https://raw.githubusercontent.com/unbill-project/unbill/main/altstore-source.json
```

The source lists the native SwiftUI Apple app as `unbill Apple`, with bundle
identifier `computer.unbill.apple`, and retains the legacy Tauri app
`computer.unbill` with its historical releases. The native app is featured.
Native release URLs pin `unbill-apple-ios.ipa` to a GitHub release tag; legacy
versions retain their `unbill-ios.ipa` URLs. The apps have separate version
histories because their bundle identifiers differ.

The source-level and app-level `tintColor` is `#0f766e`,
derived from the `--bg-accent` CSS custom property used by the native UI.

The `iconURL` points to `crates/unbill-tauri/icons/icon.png`
served through the raw GitHub content URL on the `main` branch.

## Updating for a new release

After the IPA is published, inspect the actual release asset and add a version
object at the beginning of the matching app's `versions` array. Copy its version,
bundle identifier, minimum iOS version, and privacy usage descriptions from the
IPA, and use the actual download size and release date. Native version 0.0.6
requires iOS 17.0 and declares camera and local-network usage. Its IPA is unsigned
and has no embedded provisioning profile or signing entitlements.
Keep previous versions and the legacy app entry for compatible older systems.

## AltStore source format summary

Required source keys: `name`, `apps`.
Required app keys: `name`, `bundleIdentifier`, `developerName`,
`localizedDescription`, `iconURL`, `versions`, `appPermissions`.
Required version keys: `version`, `date`, `downloadURL`, `size`.
Optional but recommended: `subtitle`, `tintColor`, `category`, `screenshots`, `news`.

`appPermissions` must list all entitlements and privacy usage descriptions.
AltStore checks them against the downloaded IPA
and refuses to install apps whose declared permissions do not match.
