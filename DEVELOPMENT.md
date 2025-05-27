# Development Guide

Simple development setup for the Big Two game backend.

## 🚀 **Quick Start**

```bash
# 1. Clone and build
git clone <repo>
cd bigtwo
cargo check  # ✅ Compiles without any setup

# 2. Run tests
cargo test   # ✅ Unit tests work without database

# 3. For runtime (when you want to actually run the server):
export DATABASE_URL="postgresql://user:pass@localhost/bigtwo"
cargo run
```

## 🎯 **Development Philosophy**

**Keep it simple:**
- ✅ Code compiles without external dependencies
- ✅ Unit tests run without database setup  
- ✅ Only need database when actually running the server
- ✅ No complex build scripts or cache management

## 🔧 **Database Setup (Only for Running)**

When you're ready to actually run the server:

```bash
# 1. Start PostgreSQL (however you prefer)
brew services start postgresql  # macOS
# or use Docker, etc.

# 2. Create database
createdb bigtwo

# 3. Set environment variable
export DATABASE_URL="postgresql://user:pass@localhost/bigtwo"

# 4. Run migrations
sqlx migrate run

# 5. Start server
cargo run
```

## 📝 **Adding Database Changes**

```bash
# 1. Create migration
sqlx migrate add your_change_name

# 2. Edit the generated SQL file in migrations/

# 3. Update your Rust structs/queries

# 4. Test compilation
cargo check  # ✅ Still works without database

# 5. When ready to test runtime:
sqlx migrate run
cargo run
```

## 🧪 **Testing Strategy**

- **Unit tests**: Use mock repositories (no database needed)
- **Integration tests**: Use test database (marked with `#[ignore]`)
- **Manual testing**: Use development database

## 🎁 **Benefits of This Approach**

- ✅ **Fast onboarding** - new developers can start coding immediately
- ✅ **CI/CD friendly** - builds work without database setup
- ✅ **Simple mental model** - database is only needed for runtime
- ✅ **Still type-safe** - Rust's type system catches most errors
- ✅ **Flexible** - use any PostgreSQL setup you prefer 