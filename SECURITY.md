# Security policy

## Supported version

baseCreator is a library used by LightCreator and MillCreator. Only the newest tagged release receives security fixes.

| Version | Supported |
|---------|-----------|
| v0.3.0 (latest) | yes |
| older | no, please update |

## Reporting a vulnerability

Please do not open a public issue for a security problem. Use GitHub's private vulnerability reporting:
<https://github.com/michalsarna/baseCreator/security/advisories/new>

Include the version, your operating system, what you did and what happened. Files that trigger the problem
(an SVG, PDF / Illustrator, image or scene file) are very helpful. You can expect an answer within a few days;
a fix is released as a new tag, and the apps are updated to it.

## What counts

baseCreator reads files from other people (SVG, PDF / Illustrator, bitmaps, clipart, scene files) and downloads
pictures from the web in the clipart gallery. Reports about crashes, hangs or memory use when reading a crafted file,
and about anything that could write or fetch something the user did not ask for, are in scope.

A problem that only shows in an app (for example the laser or the mill control) belongs in that app's repository:
<https://github.com/michalsarna/LightCreator/security/advisories/new>.
