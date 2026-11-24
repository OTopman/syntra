//! # Syntra Mail CLI Subsystem
//!
//! Provides developer tools for inspecting RFC 5322 `.eml` files and testing SMTP relays.

use clap::Subcommand;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::exit;

#[derive(Subcommand, Debug, Clone)]
pub enum MailCommand {
    /// Parse and inspect an RFC 5322 MIME .eml message file
    Parse {
        /// Path to the .eml file to inspect
        file: PathBuf,
    },
    /// Verify connectivity and authentication with an SMTP relay
    Verify {
        /// SMTP relay hostname (e.g. smtp.mailgun.org)
        #[arg(long)]
        host: String,
        /// SMTP TCP port (conventionally 587 or 25)
        #[arg(long, default_value = "587")]
        port: u16,
        /// Optional authentication username
        #[arg(long)]
        user: Option<String>,
        /// Optional authentication password
        #[arg(long)]
        pass: Option<String>,
    },
}

pub fn handle_mail_command(cmd: MailCommand) {
    match cmd {
        MailCommand::Parse { file } => parse_eml_file(&file),
        MailCommand::Verify {
            host,
            port,
            user,
            pass,
        } => verify_smtp_relay(&host, port, user.as_deref(), pass.as_deref()),
    }
}

fn parse_eml_file(path: &PathBuf) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Error reading '{}': {}", path.display(), e);
            exit(1);
        }
    };

    println!("============================================================");
    println!("✉️  Syntra MIME Email Inspector: {}", path.display());
    println!("============================================================");

    let (headers, body) = match content.find("\r\n\r\n") {
        Some(idx) => (&content[..idx], &content[idx + 4..]),
        None => match content.find("\n\n") {
            Some(idx) => (&content[..idx], &content[idx + 2..]),
            None => (content.as_str(), ""),
        },
    };

    let mut from = "N/A";
    let mut to = "N/A";
    let mut subject = "N/A";
    let mut date = "N/A";
    let mut content_type = "text/plain";
    let mut message_id = "N/A";

    for line in headers.lines() {
        let trimmed = line.trim();
        if let Some(idx) = trimmed.find(':') {
            let key = trimmed[..idx].trim().to_lowercase();
            let val = trimmed[idx + 1..].trim();
            match key.as_str() {
                "from" => from = val,
                "to" => to = val,
                "subject" => subject = val,
                "date" => date = val,
                "content-type" => content_type = val,
                "message-id" => message_id = val,
                _ => {}
            }
        }
    }

    println!("  From:         {from}");
    println!("  To:           {to}");
    println!("  Subject:      {subject}");
    println!("  Date:         {date}");
    println!("  Message-ID:   {message_id}");
    println!("  Content-Type: {content_type}");
    println!("  Payload Size: {} bytes", body.len());

    let has_attachments = content_type.to_lowercase().contains("multipart/mixed");
    if has_attachments {
        println!("  Attachments:  Detected multipart/mixed attachments container");
    }

    println!("\n--- Message Body Preview (first 250 chars) ---");
    let preview: String = body.chars().take(250).collect();
    println!("{}", preview.trim());
    if body.len() > 250 {
        println!("... [truncated]");
    }
    println!("============================================================");
}

fn verify_smtp_relay(host: &str, port: u16, user: Option<&str>, pass: Option<&str>) {
    println!("🔍 Testing connection to SMTP relay {host}:{port}...");

    let mut stream = match TcpStream::connect((host, port)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to connect to {host}:{port}: {e}");
            exit(1);
        }
    };

    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).unwrap_or(0);
    let banner = String::from_utf8_lossy(&buf[..n]);
    if !banner.starts_with("220") {
        eprintln!("❌ Unexpected SMTP banner: {}", banner.trim());
        exit(1);
    }
    println!(
        "  ✓ Received SMTP Greeting: {}",
        banner.lines().next().unwrap_or("")
    );

    let _ = stream.write_all(b"EHLO localhost\r\n");
    let n = stream.read(&mut buf).unwrap_or(0);
    let ehlo_res = String::from_utf8_lossy(&buf[..n]);
    if !ehlo_res.starts_with("250") {
        eprintln!("❌ EHLO rejected: {}", ehlo_res.trim());
        exit(1);
    }
    println!("  ✓ Handshake EHLO accepted");

    if let (Some(u), Some(p)) = (user, pass) {
        println!("  • Authenticating as '{u}'...");
        let _ = stream.write_all(b"AUTH LOGIN\r\n");
        let n = stream.read(&mut buf).unwrap_or(0);
        let auth_res = String::from_utf8_lossy(&buf[..n]);
        if auth_res.starts_with("334") {
            // Send base64 user
            let u_b64 = to_base64(u.as_bytes());
            let _ = stream.write_all(format!("{u_b64}\r\n").as_bytes());
            let _ = stream.read(&mut buf);

            // Send base64 pass
            let p_b64 = to_base64(p.as_bytes());
            let _ = stream.write_all(format!("{p_b64}\r\n").as_bytes());
            let n = stream.read(&mut buf).unwrap_or(0);
            let p_res = String::from_utf8_lossy(&buf[..n]);
            if p_res.starts_with("235") {
                println!("  ✓ Authentication successful (235 Authentication succeeded)");
            } else {
                eprintln!("❌ Authentication failed: {}", p_res.trim());
                exit(1);
            }
        } else {
            eprintln!("⚠️ Server does not accept AUTH LOGIN: {}", auth_res.trim());
        }
    }

    let _ = stream.write_all(b"QUIT\r\n");
    println!("✅ SMTP relay verification completed successfully!");
}

fn to_base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as usize;
        let b1 = if i + 1 < data.len() {
            data[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < data.len() {
            data[i + 2] as usize
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(TABLE[(triple >> 18) & 63] as char);
        out.push(TABLE[(triple >> 12) & 63] as char);
        if i + 1 < data.len() {
            out.push(TABLE[(triple >> 6) & 63] as char);
        } else {
            out.push('=');
        }
        if i + 2 < data.len() {
            out.push(TABLE[triple & 63] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}
