import React, { useState } from "react";

interface Props {
  onLogin: (apiKey: string) => void;
  error?: string;
}

export function LoginPage({ onLogin, error }: Props) {
  const [key, setKey] = useState("");

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="bg-white p-8 rounded-xl shadow-md w-96">
        <h1 className="text-2xl font-bold mb-2 text-center">MemFlow</h1>
        <p className="text-gray-500 text-sm mb-6 text-center">Enter your API key to continue</p>

        {error && <div className="bg-red-50 text-red-600 p-3 rounded mb-4 text-sm">{error}</div>}

        <form onSubmit={(e) => { e.preventDefault(); onLogin(key); }}>
          <input type="password" placeholder="sk-..." value={key}
            onChange={(e) => setKey(e.target.value)}
            className="w-full p-3 border rounded-lg mb-4 font-mono text-sm" />
          <button type="submit" disabled={!key}
            className="w-full py-3 bg-blue-600 text-white rounded-lg font-medium hover:bg-blue-700 disabled:opacity-50">
            Sign In
          </button>
        </form>
      </div>
    </div>
  );
}
