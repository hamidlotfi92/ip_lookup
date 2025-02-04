# IP Lookup

IP Lookup is a high-performance IP range lookup service built with Rust and Axum. It provides a fast and efficient way to query information about IP addresses, including their range, ASN, and ISP.

## Features

- Fast IP lookup using a custom hashmap implementation
- Support for both IPv4 and IPv6 addresses
- Single IP lookup and bulk IP lookup endpoints
- File-based configuration with automatic updates
- Response caching for improved performance
- Asynchronous operation for high concurrency

## Prerequisites

- Rust (latest stable version)
- Cargo (comes with Rust)

## Installation

1. Clone the repository:

git clone git@github.com:hamidlotfi92/ip_looup.git
cd ip_lookup


2. Build the project:

cargo build --release



## Configuration

Create a `config.toml` file in the project root with the following structure:


[server]
binding_address = "127.0.0.1:3000"
file_path = "/path/to/your/ita.cfg"

Adjust the binding_address and file_path as needed.


The server provides two endpoints:
Single IP lookup: GET /single?ip=<ip_address>
Bulk IP lookup: POST /bulk with a JSON body: {"ips": ["ip1", "ip2", ...]}

API
Single IP Lookup

GET /single?ip=192.168.1.1

Bulk IP Lookup

POST /bulk
Content-Type: application/json

{
  "ips": ["192.168.1.1", "10.0.0.1"]
}


## Performance

IP Lookup is engineered for optimal performance:

- **Custom Hashmap**: Utilizes a tailored hashmap implementation for rapid IP lookups.
- **Axum Framework**: Leverages Axum for efficient and concurrent request handling.
- **Response Caching**: Implements caching to significantly boost performance for repeated queries.
- **High-Volume Capability**: Demonstrated strong performance in testing, making it ideal for high-volume IP lookup scenarios.

## Dynamic Updates

IP Lookup ensures data freshness through automatic file monitoring:

- **Real-time File Monitoring**: Continuously watches the configured IP range file for changes.
- **Instant Updates**: Automatically refreshes the internal hashmap when changes are detected.
- **Always Current**: Guarantees that lookups always use the most up-to-date IP range information.

## Robust Error Handling

The service is designed with reliability and user-friendliness in mind:

- **Comprehensive Validation**: Thoroughly checks for invalid IP addresses.
- **Informative Responses**: Provides clear and helpful error messages for addresses not found in the database.
- **Stability Focus**: Implements robust error handling to maintain service stability under various conditions.
- **Client-Friendly**: Ensures meaningful feedback to clients, facilitating easy integration and troubleshooting.

## Contributing

We welcome contributions to IP Lookup! Here's how you can help:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request
