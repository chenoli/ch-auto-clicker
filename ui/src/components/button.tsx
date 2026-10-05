import type { ButtonHTMLAttributes, ReactElement } from "react";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "danger" | "text";
}

export function Button({ variant = "secondary", className = "", type = "button", ...rest }: ButtonProps): ReactElement {
  return <button type={type} className={`button button--${variant} ${className}`} {...rest} />;
}
