ALTER TABLE hashes 
ADD run_id uuid REFERENCES runs (id);
