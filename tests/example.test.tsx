import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { Button } from '../src/components/ui/button';

describe('Button', () => {
  it('calls onClick when clicked', () => {
    const onClick = vi.fn();

    render(<Button onClick={onClick}>Run Action</Button>);

    fireEvent.click(screen.getByRole('button', { name: 'Run Action' }));

    expect(onClick).toHaveBeenCalledTimes(1);
  });
});
