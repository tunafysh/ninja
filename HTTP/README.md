# Ninja HTTP: API Server

The HTTP/GraphQL API server for Ninja, enabling remote service management and integration with your infrastructure automation tools. Built with Axum and async-graphql for high-performance, type-safe API operations.

## Overview

The HTTP server exposes Ninja's core functionality through a RESTful and GraphQL interface. Run it standalone to manage shurikens remotely, or integrate it into your existing infrastructure for programmatic service control.

## Features

* **GraphQL API**: Full query and mutation support for Ninja operations
* **REST Endpoints**: Familiar REST interface for common operations
* **Async/Await**: Built on Tokio for handling thousands of concurrent connections
* **Type Safety**: GraphQL schema ensures client and server alignment
* **Cross-Platform**: Runs on Linux, macOS, and Windows
* **Low Overhead**: Minimal resource footprint suitable for embedded scenarios

## Getting Started

### Running the Server

From the CLI, start the API server:

```bash
shurikenctl api --port 8080
```

Or build and run the HTTP crate directly:

```bash
cargo build --release
cargo run --release --bin ninja-http -- --port 8080
```

By default, the server binds to `127.0.0.1:8080`. Access the GraphQL playground at `http://localhost:8080/graphql`.

## API Endpoints

### GraphQL Endpoint

```
POST http://localhost:8080/graphql
```

GraphQL queries, mutations, and subscriptions are processed here. The interactive playground at `/graphql` allows exploring the schema and testing queries.

### REST Endpoints

#### List Shurikens

```
GET /shurikens
```

Returns a JSON array of all installed shurikens with their metadata and current state.

```bash
curl http://localhost:8080/shurikens
```

Response:
```json
[
  {
    "id": "example-service",
    "name": "Example Service",
    "version": "1.0.0",
    "state": "Running",
    "type": "native"
  }
]
```

#### Get Shuriken Status

```
GET /shurikens/{name}
```

Retrieve detailed information about a specific shuriken.

```bash
curl http://localhost:8080/shurikens/webserver
```

#### Start Shuriken

```
POST /shurikens/{name}/start
```

Start a shuriken service.

```bash
curl -X POST http://localhost:8080/shurikens/webserver/start
```

Response (success):
```json
{
  "success": true,
  "message": "Service started"
}
```

#### Stop Shuriken

```
POST /shurikens/{name}/stop
```

Stop a running shuriken service.

```bash
curl -X POST http://localhost:8080/shurikens/webserver/stop
```

#### Restart Shuriken

```
POST /shurikens/{name}/restart
```

Restart a shuriken (stop then start).

```bash
curl -X POST http://localhost:8080/shurikens/webserver/restart
```

#### Install Shuriken

```
POST /shurikens/install
Content-Type: application/json

{
  "path": "/path/to/service.shuriken"
}
```

Install a new shuriken package.

#### Remove Shuriken

```
DELETE /shurikens/{name}
```

Remove a shuriken.

```bash
curl -X DELETE http://localhost:8080/shurikens/webserver
```

#### Configure Shuriken

```
POST /shurikens/{name}/configure
```

Generate configuration from template.

```bash
curl -X POST http://localhost:8080/shurikens/webserver/configure
```

## GraphQL Schema

The GraphQL API provides a type-safe interface to the same functionality:

```graphql
type Query {
  shurikens: [Shuriken!]!
  shuriken(name: String!): Shuriken
  status(name: String!): ServiceStatus!
}

type Mutation {
  startShuriken(name: String!): OperationResult!
  stopShuriken(name: String!): OperationResult!
  restartShuriken(name: String!): OperationResult!
  installShuriken(path: String!): OperationResult!
  removeShuriken(name: String!): OperationResult!
  configureShuriken(name: String!): OperationResult!
}

type Shuriken {
  id: String!
  name: String!
  version: String!
  description: String
  state: ServiceState!
  type: ShurikenType!
}

enum ServiceState {
  Running
  Idle
  Error
}

type OperationResult {
  success: Boolean!
  message: String
  error: String
}
```

### GraphQL Query Examples

List all shurikens:

```graphql
query {
  shurikens {
    id
    name
    version
    state
  }
}
```

Get status of a specific service:

```graphql
query {
  status(name: "webserver") {
    running
    uptime
    pid
  }
}
```

Start a shuriken:

```graphql
mutation {
  startShuriken(name: "webserver") {
    success
    message
  }
}
```

## Integration Examples

### Startup Script

```bash
#!/bin/bash
# Script to monitor and restart services

NINJA_API="http://localhost:8080"

check_and_restart() {
  RESPONSE=$(curl -s "$NINJA_API/shurikens/$1")
  STATE=$(echo $RESPONSE | jq -r '.state')

  if [ "$STATE" != "Running" ]; then
    echo "Restarting $1..."
    curl -X POST "$NINJA_API/shurikens/$1/start"
  fi
}

# Check every 30 seconds
while true; do
  check_and_restart "webserver"
  check_and_restart "database"
  sleep 30
done
```

### Python Client

```python
import requests
import json

class NinjaClient:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url

    def list_shurikens(self):
        response = requests.get(f"{self.base_url}/shurikens")
        return response.json()

    def start_shuriken(self, name):
        response = requests.post(f"{self.base_url}/shurikens/{name}/start")
        return response.json()

    def stop_shuriken(self, name):
        response = requests.post(f"{self.base_url}/shurikens/{name}/stop")
        return response.json()

# Usage
client = NinjaClient()
print(client.list_shurikens())
client.start_shuriken("webserver")
```

## Configuration

The HTTP server can be configured through environment variables:

```bash
# Bind address (default: 127.0.0.1)
export NINJA_HTTP_BIND=0.0.0.0

# Port (default: 8080)
export NINJA_HTTP_PORT=8080

# Log level (default: info)
export NINJA_LOG_LEVEL=debug

# Ninja directory (default: ~/.ninja)
export NINJA_DIR=/custom/path
```

## Performance Characteristics

* Built on Tokio's async runtime for efficient concurrency
* Connection pooling and reuse
* Minimal memory footprint
* Suitable for managing hundreds of concurrent requests
* GraphQL query complexity analysis to prevent abuse

## Security Considerations

The HTTP server operates without authentication by default. For production deployments:

1. **Network Isolation**: Run behind a firewall or on internal networks only
2. **Reverse Proxy**: Use a reverse proxy (nginx, Caddy) with authentication
3. **TLS**: Enable HTTPS if exposed to untrusted networks
4. **Rate Limiting**: Implement rate limiting at the proxy level

Example nginx configuration:

```nginx
server {
    listen 443 ssl;
    server_name ninja.example.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        auth_basic "Ninja API";
        auth_basic_user_file /etc/nginx/.htpasswd;

        proxy_pass http://127.0.0.1:8080;
    }
}
```

## Development

### Build

```bash
cargo build
cargo build --release
```

### Test

```bash
cargo test
```

### Logging

Enable detailed logging:

```bash
RUST_LOG=debug cargo run --release
```

## Architecture

The HTTP server is structured as:

1. **Router**: Axum router handles request routing
2. **Handlers**: Endpoint handlers delegate to core Ninja manager
3. **GraphQL Schema**: async-graphql resolvers mirror REST endpoints
4. **Response Serialization**: Serde handles JSON serialization

All heavy lifting is delegated to the core Ninja library, keeping the HTTP server thin and focused on API exposition.

## Troubleshooting

### Port Already in Use

```bash
# Find and kill the process using the port
lsof -i :8080
kill -9 <PID>

# Or use a different port
shurikenctl api --port 9000
```

### Connection Refused

Verify the server is running and listening:

```bash
# Via netstat
netstat -tuln | grep 8080

# Via ss
ss -tuln | grep 8080

# Via lsof
lsof -i :8080
```

### Slow Responses

Enable debug logging to identify bottlenecks:

```bash
RUST_LOG=debug shurikenctl api --port 8080
```

## Documentation

- [Axum Web Framework](https://docs.rs/axum/)
- [async-graphql](https://book.async-graphql.com/)
- [GraphQL Specification](https://spec.graphql.org/)

---

**Ninja HTTP**: Bring service management to the network.
