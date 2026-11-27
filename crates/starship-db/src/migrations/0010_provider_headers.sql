-- Optional attribution headers for OpenAI-compatible / OpenRouter requests.
ALTER TABLE provider_configs ADD COLUMN http_referer TEXT;
ALTER TABLE provider_configs ADD COLUMN x_title TEXT;
