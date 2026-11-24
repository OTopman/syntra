# Networking & WebSockets

Syntra includes high-throughput networking libraries built directly into the standard library (`stdlib/net.syn`, `stdlib/http.syn`, `stdlib/web.syn`, `stdlib/crypto.syn`).

---

## 🌐 HTTP Reactor & Server

Syntra features an asynchronous HTTP server:

```syntra
import Http;

func handle_request(HttpRequest req, HttpResponse res): void {
    if req.path == "/health" {
        res.status = 200;
        res.send("{\"status\":\"ok\"}");
    } else {
        res.status = 404;
        res.send("Not Found");
    }
}

func main(): void {
    let server = new HttpServer(8080);
    server.on_request(handle_request);
    System.println("HTTP Server listening on http://127.0.0.1:8080");
    server.listen();
}
```

---

## 🔌 RFC 6455 WebSockets

Syntra provides full native RFC 6455 WebSocket client and server capabilities:

### WebSocket Server
```syntra
import Web;

func main(): void {
    let ws_server = new WebSocketServer(9001);

    ws_server.on_connect(fn(client) {
        System.println("New WebSocket client connected!");
        client.send_text("Welcome to Syntra Real-time Gateway");
    });

    ws_server.on_message(fn(client, msg) {
        System.println("Received: " + msg);
        // Echo back with prefix
        client.send_text("Echo: " + msg);
    });

    ws_server.listen();
}
```

### Features
* **RFC 6455 Handshake:** Automatic `Sec-WebSocket-Key` decoding, GUID concatenation (`258EAFA5-E914-47DA-95CA-C5AB0DC85B11`), SHA-1 hash computation, and Base64 response encoding.
* **XOR Frame Masking:** Full client frame unmasking and server frame unmasked framing.
* **Ping / Pong & Keep-Alive:** Automatic protocol control frame handling.

---

## 🔐 Cryptography (`Crypto`)

* `Crypto.sha1(string input): string`: Generates 160-bit SHA-1 hash.
* `Crypto.base64_encode(string input): string`: Encodes data to Base64 format.
* `Crypto.base64_decode(string input): string`: Decodes Base64 data back to raw UTF-8 string.
