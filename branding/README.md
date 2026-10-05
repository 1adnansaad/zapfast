# Branding

Drop your own icon here. Git ignores everything in this folder except this
file, so the icon stays on this machine. With the folder empty, the build
uses ZapFast's own icon as a placeholder.

| File | Used for | If missing |
| --- | --- | --- |
| `icon.svg` | Window, taskbar, tray and the logo inside the app; the exe's icon is drawn from it too | ZapFast's icon |
| `icon-small.svg` | The same places when drawn below 40px (tray, small taskbar), where fine detail blurs | `icon.svg` |
| `icon.ico` | The exe's icon in Explorer and shortcuts, if you want a hand-made one | drawn from `icon.svg` at 16 to 256px |

The SVG is drawn by resvg as ZapFast builds it, so it must:

- be square (its width sets the scale);
- use shapes and paths only: text must be converted to paths, and embedded
  raster images are not drawn.

After adding or changing a file, rebuild (`cargo build --release --locked`);
the build notices the change by itself. macOS and Linux packaging keep
ZapFast's icon.
