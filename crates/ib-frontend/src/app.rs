pub fn stay_tuned_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>IssueBoss</title>
  <style>
    body { font-family: system-ui, sans-serif; display: flex; justify-content: center;
           align-items: center; min-height: 100vh; margin: 0; background: #f9fafb; }
    .card { text-align: center; padding: 2rem 3rem; background: white;
            border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,.1); }
    h1 { color: #1a1a2e; margin-bottom: 0.5rem; }
    p  { color: #6b7280; }
  </style>
</head>
<body>
  <div class="card">
    <h1>IssueBoss</h1>
    <p>Stay tuned for the UI.</p>
  </div>
</body>
</html>"#
}
