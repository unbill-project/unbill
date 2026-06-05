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

The file lists one app entry with bundle identifier `computer.unbill`.
Each version object pins its `downloadURL` to a specific GitHub release tag:
`https://github.com/unbill-project/unbill/releases/download/v{version}/unbill-ios.ipa`.
The `version` field must exactly match the `CFBundleShortVersionString` in the IPA,
which Tauri reads from the `version` key in `crates/unbill-tauri/tauri.conf.json`.
AltStore verifies this match and refuses to install on mismatch.

The source-level and app-level `tintColor` is `#0f766e`,
derived from the `--bg-accent` CSS custom property used by the native UI.

The `iconURL` points to `crates/unbill-tauri/icons/icon.png`
served through the raw GitHub content URL on the `main` branch.

## Updating for a new release

Add a new version object at the top of the `versions` array.
Set `version` to match the `version` in `tauri.conf.json` (which becomes `CFBundleShortVersionString`),
`date` to the release date in ISO 8601,
`downloadURL` to `https://github.com/unbill-project/unbill/releases/download/v{version}/unbill-ios.ipa`,
`size` to the IPA file size in bytes,
and `localizedDescription` to a short changelog summary.
The topmost entry in `versions` is the one AltStore treats as the latest release.
Keep previous version objects so users on older iOS versions can still find compatible releases.

## AltStore source format summary

Required source keys: `name`, `apps`.
Required app keys: `name`, `bundleIdentifier`, `developerName`,
`localizedDescription`, `iconURL`, `versions`, `appPermissions`.
Required version keys: `version`, `date`, `downloadURL`, `size`.
Optional but recommended: `subtitle`, `tintColor`, `category`, `screenshots`, `news`.

`appPermissions` must list all entitlements and privacy usage descriptions.
AltStore checks them against the downloaded IPA
and refuses to install apps whose declared permissions do not match.
