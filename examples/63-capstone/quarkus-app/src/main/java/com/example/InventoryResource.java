package com.example;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import java.util.Map;

@Path("/inventory")
public class InventoryResource {
    // Quarkus' OpenTelemetry extension continues the incoming traceparent and
    // creates a child span automatically; we just do the "work".
    @GET
    @Produces(MediaType.APPLICATION_JSON)
    public Map<String, Object> inventory() {
        return Map.of("sku", "widget-1", "stock", 42);
    }
}
