// Minimal Java HTTP service — no framework, no OTel SDK. We instrument it
// from the outside, at the socket layer. Quarkus or Spring would look
// identical on the wire, which is the whole point.
import com.sun.net.httpserver.HttpServer;
import java.io.OutputStream;
import java.net.InetSocketAddress;

public class Server {
    public static void main(String[] args) throws Exception {
        HttpServer s = HttpServer.create(new InetSocketAddress(8080), 0);
        s.createContext("/", ex -> {
            try { Thread.sleep((long) (Math.random() * 30)); } catch (InterruptedException ignored) {}
            byte[] body = "hello from java\n".getBytes();
            ex.sendResponseHeaders(200, body.length);
            try (OutputStream os = ex.getResponseBody()) { os.write(body); }
        });
        s.setExecutor(null);
        System.out.println("java http service on :8080");
        s.start();
    }
}
