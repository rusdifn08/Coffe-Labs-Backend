-- ==============================================================================
-- NeonDB PostgreSQL Migration: Business POS & AMOLED Always-On Display (AOD)
-- Exclusive to AccountTier::Business
-- ==============================================================================

-- 1. Business Store & Kiosk Configurations
CREATE TABLE IF NOT EXISTS business_store_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    store_name VARCHAR(120) NOT NULL DEFAULT 'Coffee Labs Bar',
    cashier_pin_hash TEXT NOT NULL,
    aod_grid_columns INT NOT NULL DEFAULT 3 CHECK (aod_grid_columns BETWEEN 1 AND 8),
    aod_grid_rows INT NOT NULL DEFAULT 2 CHECK (aod_grid_rows BETWEEN 1 AND 6),
    aod_card_border_color VARCHAR(30) NOT NULL DEFAULT '#262626',
    aod_accent_color VARCHAR(30) NOT NULL DEFAULT '#D4AF37',
    aod_idle_brightness REAL NOT NULL DEFAULT 0.35 CHECK (aod_idle_brightness BETWEEN 0.05 AND 1.0),
    aod_active_brightness REAL NOT NULL DEFAULT 0.90 CHECK (aod_active_brightness BETWEEN 0.10 AND 1.0),
    pixel_shift_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Business Menu Items (Strictly Verified Transparency PNG/WEBP/SVG)
CREATE TABLE IF NOT EXISTS business_menu_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_id UUID NOT NULL REFERENCES business_store_configs(id) ON DELETE CASCADE,
    name VARCHAR(80) NOT NULL,
    description VARCHAR(240) NOT NULL DEFAULT '',
    category VARCHAR(50) NOT NULL DEFAULT 'Signature Coffee',
    price_cents BIGINT NOT NULL CHECK (price_cents >= 0),
    currency VARCHAR(10) NOT NULL DEFAULT 'IDR',
    asset_url TEXT NOT NULL,
    asset_format VARCHAR(10) NOT NULL CHECK (asset_format IN ('PNG', 'WEBP', 'SVG')),
    has_verified_transparency BOOLEAN NOT NULL DEFAULT true,
    display_order INT NOT NULL DEFAULT 0,
    is_available BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. Point of Sale (POS) Orders
CREATE TABLE IF NOT EXISTS pos_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_id UUID NOT NULL REFERENCES business_store_configs(id) ON DELETE CASCADE,
    order_number VARCHAR(40) NOT NULL,
    cashier_user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    customer_name VARCHAR(100) DEFAULT 'Customer Bar',
    subtotal_cents BIGINT NOT NULL CHECK (subtotal_cents >= 0),
    tax_cents BIGINT NOT NULL DEFAULT 0 CHECK (tax_cents >= 0),
    total_cents BIGINT NOT NULL CHECK (total_cents >= 0),
    payment_method VARCHAR(50) NOT NULL DEFAULT 'Cash',
    status VARCHAR(30) NOT NULL DEFAULT 'completed',
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. POS Order Line Items
CREATE TABLE IF NOT EXISTS pos_order_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL REFERENCES pos_orders(id) ON DELETE CASCADE,
    menu_item_id UUID REFERENCES business_menu_items(id) ON DELETE SET NULL,
    item_name VARCHAR(80) NOT NULL,
    quantity INT NOT NULL DEFAULT 1 CHECK (quantity > 0),
    unit_price_cents BIGINT NOT NULL CHECK (unit_price_cents >= 0),
    subtotal_cents BIGINT NOT NULL CHECK (subtotal_cents >= 0),
    modifiers_json JSONB DEFAULT '{}'::jsonb
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_store_owner ON business_store_configs(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_menu_store ON business_menu_items(store_id, display_order ASC);
CREATE INDEX IF NOT EXISTS idx_pos_orders_store ON pos_orders(store_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_pos_order_items_order ON pos_order_items(order_id);
