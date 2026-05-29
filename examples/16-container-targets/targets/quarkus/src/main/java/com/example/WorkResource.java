package com.example;

import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.QueryParam;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import java.util.Map;

@Path("/")
public class WorkResource {

    @GET
    @Produces(MediaType.APPLICATION_JSON)
    public Map<String, Object> root() {
        return Map.of("service", "quarkus-target", "pid", ProcessHandle.current().pid());
    }

    @GET
    @Path("/work")
    @Produces(MediaType.APPLICATION_JSON)
    public Map<String, Object> work(@QueryParam("n") @org.jboss.resteasy.reactive.RestQuery Integer n) {
        long limit = (n == null) ? 1000 : n;
        long total = 0;
        for (long i = 0; i < limit; i++) {
            total += i * i;
        }
        return Map.of("sum", total);
    }
}
