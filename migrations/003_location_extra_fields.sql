-- Add missing location fields
ALTER TABLE locations ADD COLUMN location_type TEXT;
ALTER TABLE locations ADD COLUMN notes TEXT;
ALTER TABLE locations ADD COLUMN state TEXT;
