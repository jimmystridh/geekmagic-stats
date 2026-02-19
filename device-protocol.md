# GeekMagic SmallTV Device Protocol

Reverse-engineered HTTP API documentation for GeekMagic SmallTV devices (Ultra and Pro models).
Based on firmware analysis, web UI source inspection, and live testing against a SmallTV-Ultra running firmware **Ultra-V9.0.43**.

## Overview

GeekMagic SmallTV devices are ESP8266-based miniature displays that run an embedded HTTP server on the local network. All communication is **plain HTTP** (no TLS) with **no authentication**. The device exposes a REST-like API using GET requests for both reads and writes, and multipart POST for file uploads.

### Key Characteristics

- **Transport**: HTTP/1.1 over TCP (default port 80)
- **Authentication**: None
- **Content types**: JSON responses are served as `text/plain` or `text/json` (not `application/json`)
- **Connection**: `keep-alive` with 2000ms timeout
- **No CORS headers**: The web UI is served from the device itself
- **No server identification header**: Responses don't include a `Server` header

### Device Models

| Model | Identifier | Custom Image Theme | Unique Paths |
|-------|-----------|-------------------|--------------|
| SmallTV Ultra | `ultra` | Theme 3 | `/app.json` at root |
| SmallTV Pro | `pro` | Theme 4 | `/.sys/app.json` (Pro-specific) |

Model detection: Try `GET /.sys/app.json` first. If it returns 200, the device is a Pro. Otherwise try `GET /app.json` — if that returns 200, it's an Ultra.

---

## Display Specifications

| Property | Value |
|----------|-------|
| Resolution | 240 x 240 pixels |
| Physical size | ~4cm diagonal (1.54" TFT LCD) |
| Supported image formats | JPEG, GIF |
| Maximum recommended image size | ~400 KB |
| Color depth | 16-bit (RGB565 on display, but accepts 24-bit input) |
| Minimum readable font size | 10-12px |

### Image Guidelines

- **JPEG** is preferred over PNG for upload speed (~2.5s vs ~5.8s)
- Images should be exactly **240x240 pixels** — the device does not resize
- JPEG quality of 85-92 provides a good balance of quality and file size
- High contrast colors work best on the small display (light on dark)
- The device crops from top-left if the image is larger than 240x240
- GIF animations are supported (both in `/image/` and `/gif` directories)

---

## JSON Status Endpoints

All status endpoints use `GET` and return JSON. Note that the device returns non-standard content types (`text/plain` or `text/json` instead of `application/json`), so JSON parsers should not validate content type.

### `GET /v.json` — Device Info

Returns model name and firmware version.

```json
{"m": "SmallTV-Ultra", "v": "Ultra-V9.0.43"}
```

| Field | Type | Description |
|-------|------|-------------|
| `m` | string | Model name (e.g., `"SmallTV-Ultra"`, `"SmallTV-Pro"`) |
| `v` | string | Firmware version string |

### `GET /app.json` — Application State

Returns current theme. May include brightness and current image on some firmware versions.

```json
{"theme": 3}
```

| Field | Type | Description |
|-------|------|-------------|
| `theme` | int | Active theme number (1-7) |
| `brt` | string? | Brightness 0-100 (absent on some firmware) |
| `img` | string? | Currently displayed image path (absent on some firmware) |

> **Note**: On Ultra-V9.0.43, only `theme` is returned. Older firmware versions may include `brt` and `img`.

### `GET /brt.json` — Brightness

Returns current brightness level. **May return 404 on newer firmware.**

```json
{"brt": "71"}
```

| Field | Type | Description |
|-------|------|-------------|
| `brt` | string | Brightness level 0-100 (note: returned as string, not int) |

> **Note**: Returns 404 on Ultra-V9.0.43. Brightness can still be set via `/set?brt=`, but reading it back may not work on all firmware versions.

### `GET /space.json` — Storage Info

Returns device flash storage usage.

```json
{"total": 3121152, "free": 1154572}
```

| Field | Type | Description |
|-------|------|-------------|
| `total` | int | Total storage in bytes (~3 MB) |
| `free` | int | Free storage in bytes |

### `GET /city.json` — Weather City

Returns weather location configuration.

```json
{"ct": "Gothenburg", "t": "1", "mt": "0", "cd": "Göteborg", "loc": "Gothenburg,SE"}
```

| Field | Type | Description |
|-------|------|-------------|
| `ct` | string | City name (English) |
| `cd` | string | City name (local/display) or city number |
| `loc` | string | Location string (City,Country) |
| `t` | string | Unknown (possibly type flag) |
| `mt` | string | Unknown |

### `GET /album.json` — Photo Album Settings

Returns album/slideshow configuration.

```json
{"autoplay": 0, "i_i": 5}
```

| Field | Type | Description |
|-------|------|-------------|
| `autoplay` | int | Auto-slideshow: 0 = off, 1 = on |
| `i_i` | int | Image display interval in seconds |

### `GET /config.json` — WiFi Configuration

Returns saved WiFi credentials.

```json
{"a": "MyWiFiSSID", "p": "****"}
```

| Field | Type | Description |
|-------|------|-------------|
| `a` | string | WiFi SSID |
| `p` | string | WiFi password (masked with `****` in response) |

### `GET /wifi.json?q=1` — WiFi Network Scan

Triggers a WiFi scan and returns visible networks.

```json
{
  "aps": [
    {"c": "6", "ss": "MyNetwork", "e": 1, "r": 63},
    {"c": "11", "ss": "Neighbor", "e": 1, "r": 46}
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `aps` | array | List of access points |
| `aps[].ss` | string | SSID |
| `aps[].r` | int | Signal strength (0-100, percentage) |
| `aps[].c` | string | WiFi channel |
| `aps[].e` | int | Encryption (1 = encrypted) |

### Other JSON Endpoints

These are referenced in the web UI HTML but return **404 on Ultra-V9.0.43**. They may work on other firmware versions or the Pro model:

| Endpoint | Expected Content |
|----------|-----------------|
| `GET /delay.json` | WiFi boot delay setting |
| `GET /timebrt.json` | Night mode / timed brightness |
| `GET /theme_list.json` | Auto theme switching config |
| `GET /hour12.json` | 12/24 hour format |
| `GET /ntp.json` | NTP server |
| `GET /timecolor.json` | Clock digit colors |
| `GET /font.json` | Clock font |
| `GET /colon.json` | Colon blink setting |
| `GET /dst.json` | Daylight saving time |
| `GET /daytimer.json` | Countdown timer |
| `GET /key.json` | OpenWeatherMap API key |
| `GET /unit.json` | Weather unit preferences |
| `GET /w_i.json` | Weather update interval |

---

## Set Endpoints (Commands)

All commands use `GET` requests with query parameters. Successful commands return the plain text string `OK`.

### Display Control

#### `GET /set?theme={n}` — Set Theme

Switch the active display theme.

| Theme | Name |
|-------|------|
| 1 | Weather Clock Today |
| 2 | Weather Forecast |
| 3 | Photo Album |
| 4 | Time Style 1 |
| 5 | Time Style 2 |
| 6 | Time Style 3 |
| 7 | Simple Weather Clock |

```
GET /set?theme=3    → switches to Photo Album
```

> **Custom image mode**: For the HACS integration, the device needs to be in "Photo Album" mode (theme 3 on Ultra, theme 4 on Pro) to display uploaded images.

#### `GET /set?brt={n}` — Set Brightness

Set display brightness.

| Parameter | Range | Description |
|-----------|-------|-------------|
| `brt` | -10 to 100 | Brightness level (web UI slider allows -10 to 100) |

```
GET /set?brt=75
```

> **Note**: The web UI allows values down to -10, though the logical range is 0-100.

#### `GET /set?img={path}` — Display Image

Set which image file to display. The device must be in Photo Album theme.

```
GET /set?img=/image//myphoto.jpg
```

> **Note**: The double slash (`/image//filename`) is normal — it comes from the upload directory being `/image/` and the filename being appended.

#### `GET /set?gif={path}` — Display GIF

Set which GIF to display on the weather screen's GIF area.

```
GET /set?gif=/gif/animation.gif
```

### Album Settings

#### `GET /set?i_i={seconds}&autoplay={0|1}` — Album Slideshow

Configure photo album auto-play behavior.

| Parameter | Values | Description |
|-----------|--------|-------------|
| `i_i` | integer | Seconds between image transitions |
| `autoplay` | 0 or 1 | Enable/disable automatic slideshow |

```
GET /set?i_i=10&autoplay=1    → cycle images every 10 seconds
GET /set?i_i=5&autoplay=0     → manual only, 5s interval saved
```

### Weather Settings

#### `GET /set?cd1={city}&cd2=1000` — Set Weather City

| Parameter | Description |
|-----------|-------------|
| `cd1` | City name (e.g., `Seoul`) or OpenWeatherMap city number (e.g., `1835848`) |
| `cd2` | Always `1000` (sentinel/separator value) |

```
GET /set?cd1=Stockholm&cd2=1000
GET /set?cd1=2673730&cd2=1000     → city number for Stockholm
```

#### `GET /set?w_i={minutes}` — Weather Update Interval

| Parameter | Description |
|-----------|-------------|
| `w_i` | Minutes between weather updates (default: 20) |

#### `GET /set?key={apikey}` — OpenWeatherMap API Key

Set a custom OpenWeatherMap API key for higher rate limits.

#### `GET /set?fkey={apikey}` — Forecast API Key

Set a separate API key for weather forecast data.

#### `GET /set?w_u={wind}&t_u={temp}&p_u={pressure}` — Weather Units

| Parameter | Values | Description |
|-----------|--------|-------------|
| `w_u` | `m/s`, `km/h`, `mile/h` | Wind speed unit |
| `t_u` | `°C`, `°F` | Temperature unit |
| `p_u` | `hPa`, `kPa`, `mmHg`, `inHg` | Pressure unit |

### Time / Clock Settings

#### `GET /set?hour={0|1}` — 12/24 Hour Format

| Value | Meaning |
|-------|---------|
| 0 | 24-hour format (default) |
| 1 | 12-hour format |

#### `GET /set?day={n}` — Date Format

| Value | Format |
|-------|--------|
| 1 | DD/MM/YYYY |
| 2 | YYYY/MM/DD |
| 3 | MM/DD/YYYY |
| 4 | MM/DD |
| 5 | DD/MM |

#### `GET /set?hc={hex}&mc={hex}&sc={hex}` — Clock Digit Colors

Set colors for hour, minute, and second digits. Values are URL-encoded hex colors.

```
GET /set?hc=%23FFFFFF&mc=%23FEBA01&sc=%23FF5900
```

| Parameter | Description | Default |
|-----------|-------------|---------|
| `hc` | Hour color | `#FFFFFF` (white) |
| `mc` | Minute color | `#FEBA01` (amber) |
| `sc` | Second color | `#FF5900` (orange) |

#### `GET /set?colon={0|1}` — Colon Blink

Enable or disable the blinking colon between hours and minutes.

#### `GET /set?font={1|2}` — Clock Font

| Value | Font |
|-------|------|
| 1 | Default Big Font |
| 2 | Digital Font |

#### `GET /set?ntp={server}` — NTP Server

Set a custom NTP time server (e.g., `pool.ntp.org`).

#### `GET /set?dst={0|1}` — Daylight Saving Time

Enable or disable DST adjustment (+/- 1 hour).

#### `GET /set?yr={year}&mth={month}&day={day}` — Countdown Date

Set a target date for the countdown timer feature.

### Theme Auto-Switching

#### `GET /set?theme_list={list}&sw_en={0|1}&sw_i={seconds}`

Configure automatic theme rotation.

| Parameter | Description |
|-----------|-------------|
| `theme_list` | Comma-separated enable flags for each theme (e.g., `1,0,1,0,0,0,1,0,0`) |
| `sw_en` | Enable auto-switching (0 or 1) |
| `sw_i` | Interval in seconds between theme switches |

### Network Settings

#### `GET /wifisave?s={ssid}&p={password}` — Save WiFi Credentials

Connect to a new WiFi network. The device will reboot after saving.

| Parameter | Description |
|-----------|-------------|
| `s` | WiFi SSID (URL-encoded) |
| `p` | WiFi password (URL-encoded) |

> **Warning**: After changing WiFi, the device reboots and gets a new IP from the new network. The old IP will no longer work.

#### `GET /set?delay={seconds}` — WiFi Boot Delay

Set how long the device waits after boot before connecting to WiFi. Useful when the router takes longer to boot than the device.

### Device Management

#### `GET /set?reboot=1` — Reboot Device

Reboots the device immediately.

#### `GET /set?reset=1` — Factory Reset

**Resets the device to factory defaults.** Use with extreme caution.

#### `GET /set?clear=image` — Clear All Images

Deletes all files in the `/image/` directory.

#### `GET /set?clear=gif` — Clear All GIFs

Deletes all files in the `/gif` directory.

---

## File Operations

### `POST /doUpload?dir={directory}` — Upload File

Upload a file to the device using multipart form data.

| Directory | Purpose | Accepted Formats |
|-----------|---------|-----------------|
| `/image/` | Photo album images | JPEG, GIF (240x240px) |
| `/gif` | Weather screen GIF overlay | GIF (80x80px for weather screen) |

**Request**: `multipart/form-data` with a `file` field.

```bash
curl -F 'file=@photo.jpg;type=image/jpeg' 'http://{device}/doUpload?dir=/image/'
```

**Response**: Returns an HTML file listing table on success (not JSON, not plain text).

**Known firmware bugs**:
- **SmallTV Ultra**: May return duplicate `Content-Length` headers, causing HTTP client errors
- **SmallTV Pro**: May send data after `Connection: close` header
- Both result in HTTP 400 errors that can be safely ignored — the upload succeeds despite the error

### `GET /filelist?dir={directory}` — List Files

Returns an HTML table listing all files in the specified directory. **Not JSON** — returns raw HTML `<table>` markup.

```
GET /filelist?dir=/image/
GET /filelist?dir=/gif
```

Response is an HTML table with columns: `#`, `Name`, `Size(KB)`, `Delete` button, `Set` button.

Returns the string `Empty` if the directory has no files, or `Fail` on error.

### `GET /delete?file={path}` — Delete File

Delete a specific file from the device.

```
GET /delete?file=/image//myphoto.jpg
GET /delete?file=/gif/animation.gif
```

### `GET /image//{filename}` — Download Image

Images can be downloaded directly by their path (the same path shown in the file listing).

```
GET /image//geekmagic_art1.jpg
```

---

## OTA Firmware Update

### `GET /update` — Update Page

Returns an HTML form for uploading firmware.

### `POST /update` — Upload Firmware

Upload a `.bin` or `.bin.gz` firmware file via multipart form. The form field name is `firmware`.

```bash
curl -F 'firmware=@firmware.bin' 'http://{device}/update'
```

> **Warning**: Uploading incorrect firmware can brick the device. Always verify the model before updating.

There is also a commented-out filesystem update option in the HTML (`name='filesystem'`), suggesting it may be enabled in future firmware.

---

## Web UI Pages

The device serves a built-in web configuration interface:

| Path | Purpose |
|------|---------|
| `/` or `/settings.html` | Main settings (themes, brightness, night mode, auto-switch) |
| `/network.html` | WiFi configuration and scanning |
| `/weather.html` | Weather city, units, API keys, weather GIF upload |
| `/time.html` | Clock format, colors, font, NTP, DST, countdown |
| `/image.html` | Photo album settings, image upload/crop/manage |
| `/update` | OTA firmware update |

Static assets are served from `/css/` and `/js/`:
- `/css/style.css` — Main stylesheet
- `/css/cropper.min.css` — Image cropper library styles
- `/js/settings.js` — Shared JavaScript (API calls, navigation, helpers)
- `/js/jquery.min.js` — jQuery library
- `/js/cropper.min.js` — Cropper.js for image cropping

---

## Filesystem Layout

The device's flash filesystem has the following known directories:

| Path | Contents |
|------|----------|
| `/image/` | Photo album images (JPEG, GIF at 240x240px) |
| `/gif` | Weather screen GIF overlays (80x80px) |
| `/.sys/` | System files (Pro model only — used for model detection) |

Total storage is approximately **3 MB** (3,121,152 bytes observed), of which the firmware, web UI, and system files consume roughly 1.8-1.9 MB, leaving ~1.2 MB for user images.

---

## Typical Upload-and-Display Flow

The standard sequence to display a custom image:

```
1. GET  /set?theme=3              # Switch to Photo Album mode (3 for Ultra, 4 for Pro)
2. POST /doUpload?dir=/image/     # Upload 240x240 JPEG via multipart form
3. GET  /set?img=/image//file.jpg # Tell the device to display the uploaded image
```

For the HACS integration, this is wrapped in the `upload_and_display()` method which:
1. Calls `set_theme_custom()` (selects theme 3 or 4 based on model)
2. POSTs the image to `/doUpload?dir=/image/`
3. GETs `/set?img=/image/{filename}` to activate it

---

## Error Handling Notes

- **404 responses**: Many JSON endpoints return `404` depending on firmware version. Code should handle this gracefully.
- **Malformed HTTP responses on upload**: The device firmware has known bugs where successful uploads return technically invalid HTTP. Check for status 400 with messages containing `"Duplicate Content-Length"` or `"Data after"` and treat as success.
- **Timeout**: A 30-second timeout is recommended. The device can be slow to respond, especially during uploads.
- **No response body validation**: The device returns `text/plain` for JSON endpoints. Always parse with content-type validation disabled.
- **Connection refused**: The device only supports 2.4 GHz WiFi. If it can't be reached, it may be on a different network segment or powered off.
- **Response to /set commands**: Successful set commands return the plain text string `OK`. Failures may return empty responses or non-`OK` strings.

---

## Pro vs Ultra Differences

| Feature | Ultra | Pro |
|---------|-------|-----|
| Custom image theme | 3 | 4 |
| System path | `/app.json` | `/.sys/app.json` |
| Navigation buttons | Not available | `page=1`, `page=-1`, `enter=-1` |
| Physical buttons | None (touch or app-controlled) | Physical buttons simulated via API |
| Reboot API | `/set?reboot=1` | `/set?reboot=1` |

### Pro-Only Navigation Endpoints

These simulate physical button presses on SmallTV Pro:

```
GET /set?page=1     # Next page (right button)
GET /set?page=-1    # Previous page (left button)
GET /set?enter=-1   # Enter/menu button
```

---

## Night Mode / Timed Brightness

The web UI includes a night mode feature where brightness automatically reduces during set hours. Configuration is via `/set?t1={start_hour}&t2={end_hour}&b2={night_brightness}&en={0|1}`:

| Parameter | Description |
|-----------|-------------|
| `t1` | Start hour (0-23, e.g., `22` for 10 PM) |
| `t2` | End hour (0-23, e.g., `7` for 7 AM) |
| `b2` | Night brightness level |
| `en` | Enable (1) or disable (0) |

> **Note**: The corresponding `GET /timebrt.json` endpoint returns 404 on Ultra-V9.0.43, but the set command may still work.

---

## Rate Limits and Performance

- No explicit rate limiting, but the ESP8266 is single-threaded and handles one request at a time
- Image upload takes approximately **2-3 seconds** for a typical JPEG (~20-30 KB)
- Rapid sequential requests should include small delays (~100-500ms) to avoid overwhelming the device
- The HACS integration uses a configurable refresh interval (default 10 seconds) to avoid flooding the device
- The `keep-alive` timeout is 2000ms — reuse connections where possible

---

## Discovery

GeekMagic devices create a WiFi hotspot named `GIFTV` during initial setup. Once connected to a network, they obtain an IP via DHCP and display it on screen at boot.

There is no mDNS, SSDP, or other automatic discovery mechanism. The device must be found by its IP address.
