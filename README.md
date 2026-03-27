# ⌨️ UmlautKey

> **The lightest, fastest German Umlaut engine for macOS.**

**UmlautKey** is a minimalist, high-performance background utility that brings intuitive Umlaut typing to your Mac. Built with Rust, it allows you to type German characters naturally without switching keyboard layouts.

---

## ✨ Features

- **Smart Transformation**: Instantly converts `ae`, `oe`, `ue`, `ss`, `Shift + 4($)` into `ä`, `ö`, `ü`, `ß`, `€` as you type.
- **Visual Feedback**: Dynamic Menu Bar icon (`Ä` for Enabled, `E` for Disabled).
- **Native Performance**: Zero-latency interceptor built with Rust and macOS CoreGraphics Event Taps.
- **Privacy First**: No data collection. No internet access. No logging.

---

## 🚀 Installation & Setup

### 1. Download & Install

1. Download the `UmlautKey.zip` from the Releases page.
2. Extract the zip and drag **UmlautKey.app** to your **Applications** folder.
3. Launch **UmlautKey**.

### 2. Required Permissions (Accessibility) ⚠️

Because UmlautKey functions as a global keyboard interceptor, macOS requires explicit permission to allow it to work.

**Follow these steps to enable it:**

1. Open **System Settings** (or System Preferences).
2. Go to **Privacy & Security** > **Accessibility**.
3. Click the **[+]** button at the bottom.
4. Navigate to your **Applications** folder, select **UmlautKey**, and click **Open**.
5. Ensure the toggle next to **UmlautKey** is turned **ON**.

---

## 🎮 How to Use

Once running, the app stays in your Menu Bar. You can toggle on/off by hotkey CMD + Shift + Z

| Command     | Shortcut / Sequence | Result |
| :---------- | :------------------ | :----- |
| **Type ä**  | `a` + `e`           | `ä`    |
| **Type ae** | `a` + `e` + `e`     | `ae`   |
| **Type ö**  | `o` + `e`           | `ö`    |
| **Type oe** | `o` + `e` + `e`     | `oe`   |
| **Type ü**  | `u` + `e`           | `ü`    |
| **Type ue** | `u` + `e` + `e`     | `ue`   |
| **Type ß**  | `s` + `s`           | `ß`    |
| **Type ss** | `s` + `s` + `s`     | `ss`   |
| **Type €**  | `Shift` + `4($)`    | `€`    |

---

## 🛠 For Developers

### Prerequisites

- Rust (latest stable)
- Xcode Command Line Tools
- `create-dmg` (optional, for building the installer: `brew install create-dmg`)

### Build the App Bundle

```bash
make release-mac
```
