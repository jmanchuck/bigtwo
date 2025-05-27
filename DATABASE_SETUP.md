# Database Setup for Session Management

This document explains how to set up the PostgreSQL database for proper session validation.

## Prerequisites

- PostgreSQL 12+ installed and running
- `sqlx-cli` installed: `cargo install sqlx-cli`

## Environment Variables

Set the following environment variable:

```bash
export DATABASE_URL="postgresql://username:password@localhost/bigtwo"
export JWT_SECRET="your-secret-key-change-in-production"
```

## Database Setup

1. **Create the database:**
   ```bash
   createdb bigtwo
   ```

2. **Run migrations:**
   ```bash
   sqlx migrate run
   ```

3. **Verify the setup:**
   ```bash
   psql $DATABASE_URL -c "\dt"
   ```

   You should see the `user_sessions` table listed.

## Session Validation Flow

The new implementation provides **proper session validation** that was missing in the original JWT-only approach:

### Before (Security Gap)
- ✅ JWT signature validation
- ✅ JWT expiration check  
- ❌ **No session storage validation**
- ❌ **No session revocation capability**
- ❌ **No protection against stolen tokens**

### After (Secure)
- ✅ JWT signature validation
- ✅ JWT expiration check
- ✅ **Database session validation**
- ✅ **Session revocation support**
- ✅ **Active session tracking**
- ✅ **Expired session cleanup**

## How It Works

1. **Session Creation:**
   - Generate UUID and username
   - Store session in `user_sessions` table
   - Create JWT token with session ID
   - Return JWT to client

2. **Session Validation:**
   - Validate JWT structure and signature
   - Extract session ID from JWT claims
   - **Check if session exists in database**
   - **Verify session hasn't expired**
   - Allow request if both checks pass

3. **Session Revocation:**
   - Delete session from database
   - JWT becomes invalid even if not expired

## API Changes

The session validation is now **properly secure** but requires database connectivity:

- `POST /session` - Creates session in database + returns JWT
- Session validation middleware now checks database
- Sessions can be revoked by deleting from database
- Expired sessions are automatically cleaned up

## Migration Notes

This fixes the critical security vulnerability where JWT tokens were only validated cryptographically but not against any session store, making it impossible to revoke sessions or track active users. 