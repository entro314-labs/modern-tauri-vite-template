import type { ButtonHTMLAttributes } from 'react';

type ButtonVariant = 'primary' | 'secondary' | 'ghost';

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
};

const variantClassMap: Record<ButtonVariant, string> = {
  primary:
    'bg-[var(--color-accent)] text-[var(--color-accent-foreground)] hover:opacity-90 focus-visible:ring-[var(--color-accent)]',
  secondary:
    'bg-[var(--color-panel)] text-[var(--color-panel-foreground)] border border-[var(--color-border)] hover:bg-[var(--color-background)] focus-visible:ring-[var(--color-accent)]',
  ghost:
    'bg-transparent text-[var(--color-foreground)] hover:bg-[var(--color-panel)] focus-visible:ring-[var(--color-accent)]',
};

const classNames = (...parts: Array<string | undefined>): string => parts.filter(Boolean).join(' ');

export function Button({ className, variant = 'primary', type = 'button', ...props }: ButtonProps) {
  return (
    <button
      type={type}
      className={classNames(
        'rounded-md px-3 py-2 text-sm font-medium transition focus-visible:outline-none focus-visible:ring-2 disabled:cursor-not-allowed disabled:opacity-50',
        variantClassMap[variant],
        className,
      )}
      {...props}
    />
  );
}
