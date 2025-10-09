DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = 'public' AND table_name = 'records' AND column_name = 'received_at'
  ) THEN
    ALTER TABLE records ADD COLUMN received_at timestamp default null;
    -- Copy fields from files to records
    UPDATE records
    SET received_at = files.received_at
    FROM files
    WHERE records.file_id = files.id;
  END IF;
END $$;
