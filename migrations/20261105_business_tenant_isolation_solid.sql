-- ==============================================================================
-- NeonDB PostgreSQL Migration: Business Multi-Tenant Isolation & SOLID Services
-- Strict Store Scoping for AccountTier::Business vs AccountTier::Pro
-- ==============================================================================

-- 1. Business Commercial Inventory Items (Beans, Dairy, Syrups, Packaging)
CREATE TABLE IF NOT EXISTS business_inventory_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_id UUID NOT NULL REFERENCES business_store_configs(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    sku VARCHAR(50),
    category VARCHAR(50) NOT NULL DEFAULT 'Coffee Beans',
    stock_quantity DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    unit VARCHAR(20) NOT NULL DEFAULT 'gram',
    min_reorder_level DOUBLE PRECISION NOT NULL DEFAULT 100.0,
    cost_per_unit_cents BIGINT NOT NULL DEFAULT 0,
    supplier_name VARCHAR(120),
    last_restocked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Business Cashier Shifts & Cash Register Balances
CREATE TABLE IF NOT EXISTS business_cashier_shifts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_id UUID NOT NULL REFERENCES business_store_configs(id) ON DELETE CASCADE,
    cashier_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    cashier_name VARCHAR(100) NOT NULL,
    shift_name VARCHAR(50) NOT NULL DEFAULT 'Morning Shift',
    opened_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at TIMESTAMPTZ,
    opening_cash_cents BIGINT NOT NULL DEFAULT 0,
    expected_cash_cents BIGINT NOT NULL DEFAULT 0,
    actual_cash_cents BIGINT,
    cash_difference_cents BIGINT,
    total_orders_count INT NOT NULL DEFAULT 0,
    total_sales_cents BIGINT NOT NULL DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'open',
    notes TEXT
);

-- 3. Business Dining & Bar Tables (Seating Layout & Dine-in Tracking)
CREATE TABLE IF NOT EXISTS business_dining_tables (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_id UUID NOT NULL REFERENCES business_store_configs(id) ON DELETE CASCADE,
    table_number VARCHAR(20) NOT NULL,
    section VARCHAR(50) NOT NULL DEFAULT 'Indoor Bar',
    capacity INT NOT NULL DEFAULT 2,
    status VARCHAR(20) NOT NULL DEFAULT 'available',
    active_order_id UUID REFERENCES pos_orders(id) ON DELETE SET NULL,
    qr_code_token VARCHAR(64) UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_store_table_number UNIQUE (store_id, table_number)
);

-- 4. Business Staff & Baristas Management (Segregated per store)
CREATE TABLE IF NOT EXISTS business_staff_members (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_id UUID NOT NULL REFERENCES business_store_configs(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    full_name VARCHAR(100) NOT NULL,
    role_title VARCHAR(50) NOT NULL DEFAULT 'Barista',
    passcode_pin_hash VARCHAR(255) NOT NULL,
    can_give_discounts BOOLEAN NOT NULL DEFAULT false,
    can_void_orders BOOLEAN NOT NULL DEFAULT false,
    can_view_reports BOOLEAN NOT NULL DEFAULT false,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Composite Indexes for strict Tenant Query Performance & Zero-Leak Isolation
CREATE INDEX IF NOT EXISTS idx_inventory_store_cat ON business_inventory_items(store_id, category);
CREATE INDEX IF NOT EXISTS idx_inventory_store_sku ON business_inventory_items(store_id, sku);
CREATE INDEX IF NOT EXISTS idx_shifts_store_status ON business_cashier_shifts(store_id, status);
CREATE INDEX IF NOT EXISTS idx_shifts_store_opened ON business_cashier_shifts(store_id, opened_at DESC);
CREATE INDEX IF NOT EXISTS idx_tables_store ON business_dining_tables(store_id, section);
CREATE INDEX IF NOT EXISTS idx_staff_store ON business_staff_members(store_id, is_active);
