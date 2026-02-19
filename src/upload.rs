use std::io::Cursor;

use anyhow::{Context, Result};
use image::RgbaImage;
use reqwest::blocking::multipart;

fn encode_jpeg(img: &RgbaImage) -> Result<Vec<u8>> {
    let rgb = image::DynamicImage::ImageRgba8(img.clone()).into_rgb8();
    let mut jpeg_buf = Cursor::new(Vec::new());
    rgb.write_to(&mut jpeg_buf, image::ImageFormat::Jpeg)?;
    Ok(jpeg_buf.into_inner())
}

fn upload_file(client: &reqwest::blocking::Client, base: &str, filename: &str, jpeg_bytes: Vec<u8>) -> Result<()> {
    let part = multipart::Part::bytes(jpeg_bytes)
        .file_name(filename.to_string())
        .mime_str("image/jpeg")?;
    let form = multipart::Form::new().part("file", part);

    let resp = client
        .post(format!("{base}/doUpload?dir=/image/"))
        .multipart(form)
        .send();

    match resp {
        Ok(_) => {}
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Duplicate Content-Length") || msg.contains("Data after") || msg.contains("invalid content-length") {
            } else {
                return Err(e).context("upload failed");
            }
        }
    }
    Ok(())
}

fn make_client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?)
}

pub fn upload_and_display(host: &str, img: &RgbaImage) -> Result<()> {
    let base = format!("http://{host}");
    let client = make_client()?;

    upload_file(&client, &base, "stats.jpg", encode_jpeg(img)?)?;

    client.get(format!("{base}/set?theme=3")).send().context("failed to set theme")?;
    client.get(format!("{base}/set?img=/image//stats.jpg")).send().context("failed to set image")?;

    Ok(())
}

pub fn upload_album(host: &str, images: &[(&str, &RgbaImage)]) -> Result<()> {
    let base = format!("http://{host}");
    let client = make_client()?;

    // Clear existing images
    let resp = client.get(format!("{base}/filelist?dir=/image/")).send()?;
    let body = resp.text().unwrap_or_default();
    for line in body.lines() {
        let name = line.trim();
        if !name.is_empty() && name.ends_with(".jpg") {
            let _ = client.get(format!("{base}/del?path=/image//{name}")).send();
        }
    }

    for (filename, img) in images {
        upload_file(&client, &base, filename, encode_jpeg(img)?)?;
    }

    client.get(format!("{base}/set?theme=3")).send().context("failed to set theme")?;
    if let Some((first, _)) = images.first() {
        client.get(format!("{base}/set?img=/image//{first}")).send().context("failed to set image")?;
    }

    // Enable autoplay with 10s interval
    client.get(format!("{base}/set?i_i=10&autoplay=1")).send().context("failed to enable autoplay")?;

    Ok(())
}
