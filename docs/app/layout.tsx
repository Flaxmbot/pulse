import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Pulse Documentation",
  description:
    "Pulse language documentation: tutorials, grammar reference, examples, guides, and advanced compiler/runtime docs."
};

export default function RootLayout({
  children
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}

