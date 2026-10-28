#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["playwright==1.64.1"]
# ///
"""Regenerate the README screenshots and the GitHub social preview.

The app runs in a plain browser against its mock backend (app/src/lib/mock.ts),
so every name, serial and frequency in the images is invented. Uses the
installed Google Chrome; no Playwright browser download.

Usage:
  cd app && npm run dev            # in another terminal
  scripts/screenshots/capture.py   # writes docs/images/*.png

Then compress: pngquant --quality 80-95 --strip --force --ext .png docs/images/*.png
"""
import base64
import re
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs" / "images"
URL = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:1420/"

FRAME_CSS = """body{margin:0;background:transparent}
.pad{padding:28px 40px 56px;display:inline-block}
.win{width:1000px;border-radius:12px;overflow:hidden;border:1px solid rgba(0,0,0,.18);
box-shadow:0 22px 50px rgba(0,0,0,.28),0 4px 12px rgba(0,0,0,.12);background:#fff}
.bar{height:30px;background:#ECECEC;border-bottom:1px solid #D4D4D4;position:relative;
font:600 13px -apple-system,"SF Pro Text",sans-serif;color:#4a4a4a;text-align:center;line-height:30px}
.dots{position:absolute;left:12px;top:9px;display:flex;gap:8px}
.dots i{width:12px;height:12px;border-radius:50%;display:block}
img{display:block;width:1000px}"""

SOCIAL_CSS = """html,body{margin:0;width:1280px;height:640px;overflow:hidden}
body{background:radial-gradient(120% 140% at 20% 10%,#18978C 0%,#0E5F58 45%,#05312E 100%);
font-family:"SF Pro Display","SF Pro",-apple-system,Helvetica,Arial,sans-serif;color:#fff;
display:flex;align-items:center;box-sizing:border-box;padding:0 96px 0 72px;gap:56px}
.pager{height:540px;flex:none}
h1{font-size:132px;font-weight:800;letter-spacing:-4px;margin:0 0 8px;line-height:1}
.tag{font-size:40px;font-weight:500;line-height:1.22;margin:0 0 30px;color:#E6F4F2}
.plat{font-size:30px;font-weight:600;color:#F59E0B;margin:0 0 44px;letter-spacing:.3px}
.foot{font-size:24px;font-weight:500;color:#9FD3CD}"""


def capture_app(browser):
    """Three views of the mock app, raw, at 2x."""
    shots = {}
    pg = browser.new_page(viewport={"width": 1280, "height": 760}, device_scale_factor=2, color_scheme="light")
    pg.goto(URL)
    pg.wait_for_timeout(1200)
    pg.get_by_role("button", name="Open latest backup").click()
    pg.wait_for_timeout(1000)

    pg.get_by_role("button", name="1 TG Scan Fire").click()
    pg.wait_for_timeout(600)
    pg.mouse.move(0, 0)
    shots["zones"] = pg.screenshot()

    # Scroll the systems editor down to the control channels and talkgroup table.
    pg.get_by_role("button", name="P25 systems").click()
    pg.wait_for_timeout(800)
    pg.get_by_text("Control channels (4)").evaluate(
        "e=>{e.scrollIntoView({block:'start'}); let s=e; while(s&&s.scrollHeight<=s.clientHeight+1) s=s.parentElement; if(s) s.scrollTop-=24}"
    )
    pg.wait_for_timeout(500)
    pg.mouse.move(0, 0)
    shots["systems"] = pg.screenshot()

    # Two renames, then the write dialog's checklist.
    pg.get_by_role("button", name="Zones & knobs").click()
    pg.wait_for_timeout(500)
    pg.get_by_role("button", name="1 TG Scan Fire").click()
    pg.wait_for_timeout(300)
    pg.locator("main input").nth(1).fill("Fire Dispatch")
    pg.get_by_role("button", name="3 TG Scan Police").click()
    pg.wait_for_timeout(300)
    pg.locator("main input").nth(1).fill("PD North/South")
    pg.get_by_role("button", name="Write to pager").click()
    pg.wait_for_timeout(1500)
    pg.mouse.move(0, 0)
    shots["write"] = pg.screenshot()
    pg.close()
    return shots


def frame(browser, shots):
    """Put each shot in a macOS-style window with a shadow, on a transparent background."""
    pg = browser.new_page(viewport={"width": 1200, "height": 900}, device_scale_factor=2)
    for name, png in shots.items():
        src = "data:image/png;base64," + base64.b64encode(png).decode()
        dots = "".join(f'<i style="background:{c}"></i>' for c in ("#FF5F57", "#FEBC2E", "#28C840"))
        pg.set_content(
            f'<style>{FRAME_CSS}</style><div class="pad"><div class="win">'
            f'<div class="bar"><span class="dots">{dots}</span>Knobset</div><img src="{src}"></div></div>'
        )
        pg.wait_for_timeout(200)
        pg.locator(".pad").screenshot(path=OUT / f"{name}.png", omit_background=True)
    pg.close()


def social(browser):
    """The 1280x640 card for Settings > General > Social preview, drawn from the app icon."""
    svg = (ROOT / "app/src-tauri/icons/app-icon.svg").read_text()
    defs = re.search(r"<defs>.*?</defs>", svg, re.S).group(0)
    body = re.search(r"(<g transform=.*)</svg>", svg, re.S).group(1)
    pager = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="250 100 524 860" class="pager">{defs}{body}</svg>'
    pg = browser.new_page(viewport={"width": 1280, "height": 640}, device_scale_factor=1)
    pg.set_content(
        f'<meta charset="utf-8"><style>{SOCIAL_CSS}</style>{pager}<div>'
        "<h1>Knobset</h1>"
        '<p class="tag">Program and back up Unication<br>G5 and G&#8209;series pagers</p>'
        '<p class="plat">macOS · iPad · Linux · Windows</p>'
        '<p class="foot">Open source (MPL-2.0) · github.com/harpchad/knobset</p></div>'
    )
    pg.wait_for_timeout(300)
    pg.screenshot(path=OUT / "social-preview.png")
    pg.close()


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as p:
        browser = p.chromium.launch(channel="chrome")
        frame(browser, capture_app(browser))
        social(browser)
        browser.close()
    print(f"wrote {', '.join(sorted(f.name for f in OUT.glob('*.png')))} to {OUT}")


if __name__ == "__main__":
    main()
