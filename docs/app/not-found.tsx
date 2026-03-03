import Link from "next/link";

export default function NotFound() {
  return (
    <div className="home">
      <section className="hero">
        <h1>Page Not Found</h1>
        <p>The requested documentation page does not exist.</p>
        <Link className="primary" href="/docs/start/overview">
          Go to Docs Overview
        </Link>
      </section>
    </div>
  );
}

