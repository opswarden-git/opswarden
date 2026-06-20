# How to Contribute to OpsWarden

Welcome to OpsWarden! We are thrilled to have you contribute to our open-source project. This guide will help you get started with the codebase and our development workflow.

## 🛠️ Technology Stack

OpsWarden is composed of two main components:

1. **Backend Server (`server/`)**: Built in **Rust** using the [Axum](https://github.com/tokio-rs/axum) web framework and [SQLx](https://github.com/launchbadge/sqlx) for PostgreSQL interactions. It features an event-driven architecture, WebSockets for real-time synchronization, and robust security.
2. **Frontend Client (`client-web/`)**: Built in **TypeScript** using **Next.js 15**, React, and Tailwind CSS. It uses TanStack Query for state management and fetching, and `react-use-websocket` for real-time updates.

## 🚀 Setting Up the Development Environment

### Prerequisites

- [Nix](https://nixos.org/download.html) (with flakes enabled) - We strongly recommend using Nix to ensure your environment perfectly matches our CI.
- Docker and Docker Compose (for the PostgreSQL database).

### Getting Started

1. **Clone the repository**:

   ```bash
   git clone https://github.com/your-org/opswarden.git
   cd opswarden/opswarden-app
   ```

2. **Start the database**:

   ```bash
   docker compose up -d
   ```

3. **Enter the Nix development shell**:
   This will automatically install Rust, Node.js, and all required tools.

   ```bash
   nix develop
   ```

4. **Initialize the database**:

   ```bash
   cargo sqlx database setup
   ```

5. **Start the applications**:
   You will need two separate terminal windows (both in `nix develop`).
   - **Backend**:
     ```bash
     cargo run -p opswarden-server
     ```
   - **Frontend**:
     ```bash
     cd client-web
     npm run dev
     ```

## 📝 Coding Standards

### Rust (Backend)

- We strictly enforce formatting and linting.
- Before committing, run:
  ```bash
  cargo fmt
  cargo clippy -- -D warnings
  ```
- All tests must pass:
  ```bash
  cargo test
  ```

### TypeScript (Frontend)

- We use Prettier for formatting and ESLint for linting.
- Run the following before committing:
  ```bash
  cd client-web
  npm run lint
  npm run format
  ```
- For styling, we use Tailwind CSS. Ensure your designs align with the existing `glass` aesthetic.

## 🤝 Contribution Workflow

1. Fork the repository and create a new branch from `main` (e.g., `feature/awesome-new-feature` or `fix/issue-123`).
2. Write your code, following the coding standards above.
3. Write or update tests to cover your changes.
4. Ensure the CI passes locally (`cargo test`, `cargo clippy`, `npm run lint`).
5. Submit a Pull Request! Please include a clear description of your changes and why they are needed.

Thank you for helping us build OpsWarden!
