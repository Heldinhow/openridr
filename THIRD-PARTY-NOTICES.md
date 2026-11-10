# Third-party notices

OpenRidr distributes the following third-party components inside the app bundle. Each section names its license; the full text of each license follows its components.

The components in the first group (netcoredbg, Roslyn, dbgshim, csharp-ls and the private
.NET runtime) are MIT-licensed.

## netcoredbg 3.2.0-1092 (osx-arm64)

Bundled from the official Samsung release asset `netcoredbg-osx-arm64.zip`
(3,389,504 bytes, SHA-256
`f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea`).

Copyright (c) 2017 Samsung Electronics Co., LTD

## Microsoft.CodeAnalysis (Roslyn)

`Microsoft.CodeAnalysis.dll`, `Microsoft.CodeAnalysis.CSharp.dll`,
`Microsoft.CodeAnalysis.Scripting.dll` and
`Microsoft.CodeAnalysis.CSharp.Scripting.dll`, as shipped in the netcoredbg
asset above.

Copyright (c) Microsoft Corporation. All rights reserved.

## dbgshim and the .NET runtime

`libdbgshim.dylib` and `ManagedPart.dll`, as shipped in the netcoredbg asset
above.

Copyright (c) .NET Foundation and Contributors. All rights reserved.

## csharp-ls 0.28.0

Bundled from the official NuGet package `csharp-ls` 0.28.0 (21,643,814 bytes,
SHA-512 `a138b9c8f9de9dd6343e9b0d0c8f4d51637bba7e38606b7c2b08b1d91e8eefa39f12123057f6625f425812b4361e52841bd2856704ec127ff50144ce05170a99`),
which declares `MIT` as its license expression. `Contents/Resources/csharp-ls`
carries its runnable payload, including the F# and Roslyn assemblies the
package ships.

Copyright (c) Saulius Menkevičius and contributors

## Private .NET runtime 10.0.14 (osx-arm64)

Bundled from the official `dotnet-runtime-10.0.14-osx-arm64.tar.gz`
(33,251,887 bytes, SHA-512
`19c5781dc2f87600865886b8774a4c5851caf1f5cf3f591185b922b0e51dea6b8c0ed0bf80ae4d3be2e083db662124258743e779fab6885322d641eff52b0382`)
and contains no SDK. It runs csharp-ls only; project builds use the SDK the
project pins. Its `LICENSE.txt` and `ThirdPartyNotices.txt` ship unmodified next
to the runtime in `Contents/Resources/dotnet/`.

Copyright (c) .NET Foundation and Contributors. All rights reserved.

## MIT License

The MIT License applies to all components listed above:

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

# UI assets

## Lucide (lucide-react 1.68.0) — ISC, and MIT for icons derived from Feather

All UI icons except the file-tree glyphs listed below are drawn by `lucide-react`, bundled into the app's
JavaScript.

```text
ISC License

Copyright (c) 2026 Lucide Icons and Contributors

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.

---

The following Lucide icons are derived from the Feather project:

airplay, alert-circle, alert-octagon, alert-triangle, aperture, arrow-down-circle, arrow-down-left, arrow-down-right, arrow-down, arrow-left-circle, arrow-left, arrow-right-circle, arrow-right, arrow-up-circle, arrow-up-left, arrow-up-right, arrow-up, at-sign, calendar, cast, check, chevron-down, chevron-left, chevron-right, chevron-up, chevrons-down, chevrons-left, chevrons-right, chevrons-up, circle, clipboard, clock, code, columns, command, compass, corner-down-left, corner-down-right, corner-left-down, corner-left-up, corner-right-down, corner-right-up, corner-up-left, corner-up-right, crosshair, database, divide-circle, divide-square, dollar-sign, download, external-link, feather, frown, hash, headphones, help-circle, info, italic, key, layout, life-buoy, link-2, link, loader, lock, log-in, log-out, maximize, meh, minimize, minimize-2, minus-circle, minus-square, minus, monitor, moon, more-horizontal, more-vertical, move, music, navigation-2, navigation, octagon, pause-circle, percent, plus-circle, plus-square, plus, power, radio, rss, search, server, share, shopping-bag, sidebar, smartphone, smile, square, table-2, tablet, target, terminal, trash-2, trash, triangle, tv, type, upload, x-circle, x-octagon, x-square, x, zoom-in, zoom-out

The MIT License (MIT) (for the icons listed above)

Copyright (c) 2013-present Cole Bemis

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Material Icon Theme (`material-extensions/vscode-material-icon-theme`) — MIT

The file and folder glyphs of the Solution Explorer — and the same glyph in the tabs, the Git panel,
Find in Files and Search Everywhere — come from the **Material Icon Theme**. The SVGs are copied
**unmodified** from the theme at a pinned commit:

| Field | Value |
|---|---|
| Source | `material-extensions/vscode-material-icon-theme` (formerly `PKief/vscode-material-icon-theme`) |
| Tag | `v5.40.0` |
| Commit | `045291ff033062a25441dd297fd51546326866f2` (2026-10-26) |
| License | MIT — `LICENSE` reads "The MIT License (MIT) / Copyright (c) 2025 Material Extensions" |
| Mapping | derived from the `material-icons.json` the theme's own `generateJson` script produces from `src/core/icons/fileIcons.ts` and `folderIcons.ts` at that commit |
| Open folders | the `*-open.svg` variants are generated by the theme's own `src/scripts/svg/generateOpenFolderIcons.ts` at that commit, which swaps the `#folder` path for the theme's canonical open-folder path |

**Modified: none.** Every file in `public/file-icons/` and `public/folder-icons/` is the upstream
file byte for byte, including its `fill` colours. The theme JSON is **not** vendored and is not
loaded at runtime; the theme is not a dependency of OpenRidr, and no font is shipped.

File icons copied (30), from `icons/<name>.svg`:

`csharp`, `visualstudio`, `fsharp`, `xml`, `settings`, `json`, `xaml`, `razor`, `markdown`, `mdx`,
`yaml`, `document`, `console`, `powershell`, `css`, `sass`, `less`, `html`, `image`, `svg`, `git`,
`editorconfig`, `docker`, `license`, `readme`, `nodejs`, `tsconfig`, `nuget`, `tune`, `file`.

Folder icons copied (15, each as `<name>.svg` and `<name>-open.svg`):

`folder`, `folder-src`, `folder-test`, `folder-docs`, `folder-config`, `folder-public`,
`folder-dist`, `folder-github`, `folder-vscode`, `folder-scripts`, `folder-resource`,
`folder-controller`, `folder-class`, `folder-views`, `folder-migrations`.

Copyright (c) 2025 Material Extensions

The MIT License above applies to this component.

## Inter Variable (`@fontsource-variable/inter` 5.3.0) — SIL OFL 1.1

Latin subset of the variable font, bundled as `inter-latin-wght-normal.woff2`.

Copyright 2016 The Inter Project Authors (https://github.com/rsms/inter)

## JetBrains Mono Variable (`@fontsource-variable/jetbrains-mono` 5.3.0) — SIL OFL 1.1

Latin subset of the roman and italic variable fonts, bundled as `jetbrains-mono-latin-wght-{normal,italic}.woff2`.

Copyright 2020 The JetBrains Mono Project Authors (https://github.com/JetBrains/JetBrainsMono)

## SIL Open Font License, Version 1.1

This applies to Inter and JetBrains Mono above. The fonts are not sold on their own and are not modified
beyond the subsetting/packaging done by Fontsource.

```text
-----------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font creation
efforts of academic and linguistic communities, and to provide a free and
open framework in which fonts may be shared and improved in partnership
with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded,
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply
to any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software components as
distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to a
new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed, modify,
redistribute, and sell modified and unmodified copies of the Font
Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components,
in Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the corresponding
Copyright Holder. This restriction only applies to the primary font name as
presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created
using the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.
```
