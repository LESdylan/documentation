# Web Library Beta

## Viewing the Webpage with Cargo

To run the development server and view the webpage, use the following command:

```
cargo run --bin dev_server
```

This will start the local server. Open your browser and go to `http://localhost:8000` (or the port specified by your server) to view the webpage.

## Rebuilding the Webpage

To rebuild the webpage, run:

```
cargo run --bin doc-generator
```

This will regenerate the static files for the website.

## Other Useful Cargo Commands

- **Full clean and rebuild:**

  ```
  cargo clean && cargo build
  ```

  This removes all build artifacts and rebuilds everything from scratch.

- **Run tests:**

  ```
  cargo test
  ```

  Runs all tests in the project.

- **Build in release mode:**
  ```
  cargo build --release
  ```
  Builds the project with optimizations for production.

## Additional Information

- Make sure you have Rust and Cargo installed.
- If you encounter issues, check the documentation in `docs/` or the source code in `src/`.
