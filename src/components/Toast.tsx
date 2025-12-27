import { useEffect } from "react";
import "./Toast.css";

interface ToastProps {
  message: string;
  type?: "warning" | "error" | "info";
  onDismiss: () => void;
  duration?: number;
}

export function Toast({
  message,
  type = "warning",
  onDismiss,
  duration = 5000,
}: ToastProps) {
  useEffect(() => {
    const timer = setTimeout(onDismiss, duration);
    return () => clearTimeout(timer);
  }, [onDismiss, duration]);

  return (
    <div className={`toast toast-${type}`} role="alert">
      <span>{message}</span>
      <button onClick={onDismiss} aria-label="Dismiss">
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
  );
}
