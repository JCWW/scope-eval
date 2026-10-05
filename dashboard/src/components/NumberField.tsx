// A numeric text field that commits on blur or Enter, so typing "12.5"
// doesn't rebuild the simulation at "1", "12" and "12.".

import { TextField, type TextFieldProps } from '@mui/material';
import { useEffect, useState } from 'react';

type Props = Omit<TextFieldProps, 'value' | 'onChange' | 'error'> & {
  value: number | null | undefined;
  /** Called with the parsed number, or null when the field is cleared (if `allowEmpty`). */
  onCommit: (value: number | null) => void;
  allowEmpty?: boolean;
  min?: number;
  max?: number;
};

export function NumberField({ value, onCommit, allowEmpty, min, max, helperText, ...rest }: Props) {
  const [text, setText] = useState(value == null ? '' : String(value));
  const [problem, setProblem] = useState<string | null>(null);

  useEffect(() => {
    setText(value == null ? '' : String(value));
    setProblem(null);
  }, [value]);

  const commit = () => {
    const t = text.trim();
    if (t === '') {
      if (allowEmpty) {
        setProblem(null);
        if (value != null) onCommit(null);
      } else {
        setProblem('Enter a number');
      }
      return;
    }
    const n = Number(t);
    if (!Number.isFinite(n)) return setProblem('Not a number');
    if (min !== undefined && n < min) return setProblem(`At least ${min}`);
    if (max !== undefined && n > max) return setProblem(`At most ${max}`);
    setProblem(null);
    if (n !== value) onCommit(n);
  };

  return (
    <TextField
      {...rest}
      size="small"
      value={text}
      error={problem !== null}
      helperText={problem ?? helperText}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit();
      }}
      slotProps={{ htmlInput: { inputMode: 'decimal' } }}
    />
  );
}
