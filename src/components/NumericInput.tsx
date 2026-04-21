import { useState, useEffect, useRef } from "react";

interface NumericInputProps {
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  step?: number;
  className?: string;
  style?: React.CSSProperties;
  placeholder?: string;
  disabled?: boolean;
}

export default function NumericInput({
  value,
  onChange,
  min,
  max,
  step,
  className = "input",
  style,
  placeholder = "0.00",
  disabled,
}: NumericInputProps) {
  const [text, setText] = useState(value.toString());
  const [focused, setFocused] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  // Sync external value changes (but not while editing)
  useEffect(() => {
    if (!focused) {
      setText(value.toString());
    }
  }, [value, focused]);

  const handleFocus = () => {
    setFocused(true);
    // Select all on focus
    setTimeout(() => inputRef.current?.select(), 0);
  };

  const handleBlur = () => {
    setFocused(false);
    // Parse and validate on blur
    let num = parseFloat(text.replace(",", ".")) || 0;
    if (min !== undefined && num < min) num = min;
    if (max !== undefined && num > max) num = max;
    setText(num.toString());
    onChange(num);
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    // Allow empty, digits, dots, commas, minus
    if (val === "" || /^-?\d*[.,]?\d*$/.test(val)) {
      setText(val);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      (e.target as HTMLInputElement).blur();
    }
    // Arrow up/down
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      e.preventDefault();
      const s = step || 1;
      let num = parseFloat(text.replace(",", ".")) || 0;
      num = e.key === "ArrowUp" ? num + s : num - s;
      if (min !== undefined && num < min) num = min;
      if (max !== undefined && num > max) num = max;
      setText(num.toString());
      onChange(num);
    }
  };

  return (
    <input
      ref={inputRef}
      type="text"
      inputMode="decimal"
      className={className}
      style={style}
      value={text}
      placeholder={placeholder}
      disabled={disabled}
      onChange={handleChange}
      onFocus={handleFocus}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
    />
  );
}
