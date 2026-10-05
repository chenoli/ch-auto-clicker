import type { ButtonHTMLAttributes, ReactElement, Ref } from "react";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "danger" | "text";
  ref?: Ref<HTMLButtonElement>;
}

export function Button({ variant = "secondary", className = "", type = "button", ...rest }: ButtonProps): ReactElement {
  return <button type={type} className={`button button--${variant} ${className}`} {...rest} />;
}
