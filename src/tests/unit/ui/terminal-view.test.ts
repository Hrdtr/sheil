import TerminalView from '$lib/components/terminal-view.svelte';
import { cleanup, render } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => {
  const writeln = vi.fn();
  const dispose = vi.fn();
  const open = vi.fn();
  const loadAddon = vi.fn();

  const fit = vi.fn();
  const fitAddon = vi.fn().mockImplementation(function () {
    return { fit };
  });

  const webLinksAddon = vi.fn().mockImplementation(function () {
    return {};
  });

  const webglAddon = vi.fn().mockImplementation(function () {
    return {};
  });

  return { dispose, open, writeln, loadAddon, fit, fitAddon, webLinksAddon, webglAddon };
});

vi.mock('xterm', () => ({
  Terminal: vi.fn().mockImplementation(function () {
    return {
      dispose: mocks.dispose,
      open: mocks.open,
      writeln: mocks.writeln,
      loadAddon: mocks.loadAddon,
    };
  }),
}));

vi.mock('@xterm/addon-fit', () => ({
  FitAddon: mocks.fitAddon,
}));

vi.mock('@xterm/addon-web-links', () => ({
  WebLinksAddon: mocks.webLinksAddon,
}));

vi.mock('@xterm/addon-webgl', () => ({
  WebglAddon: mocks.webglAddon,
}));

describe('TerminalView — smoke / regression', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal(
      'ResizeObserver',
      vi.fn().mockImplementation(function () {
        return {
          observe: vi.fn(),
          unobserve: vi.fn(),
          disconnect: vi.fn(),
        };
      }),
    );
  });

  afterEach(() => {
    cleanup();
  });

  it('renders without crashing', () => {
    const { container } = render(TerminalView);
    expect(container).toBeTruthy();
  });

  it('renders a Card wrapper', () => {
    const { container } = render(TerminalView);
    const card = container.querySelector('[data-slot="card"]');
    expect(card).toBeTruthy();
  });

  it('renders "Terminal" as the Card title', () => {
    const { container } = render(TerminalView);
    const title = container.querySelector('[data-slot="card-title"]');
    expect(title).toBeTruthy();
    expect(title!.textContent).toBe('Terminal');
  });

  it('renders a container div for the terminal', () => {
    const { container } = render(TerminalView);
    const content = container.querySelector('[data-slot="card-content"]');
    expect(content).toBeTruthy();
    const div = content!.querySelector('div');
    expect(div).toBeTruthy();
    expect(div!.className).toContain('min-h-[400px]');
  });

  it('writes the welcome message', () => {
    render(TerminalView);
    expect(mocks.writeln).toHaveBeenCalledWith('Welcome to Sheil Terminal');
    expect(mocks.writeln).toHaveBeenCalledWith('');
  });

  it('loads the FitAddon', () => {
    render(TerminalView);
    expect(mocks.fitAddon).toHaveBeenCalled();
  });

  it('loads the WebLinksAddon', () => {
    render(TerminalView);
    expect(mocks.webLinksAddon).toHaveBeenCalled();
  });

  it('attempts to load the WebglAddon', () => {
    render(TerminalView);
    expect(mocks.webglAddon).toHaveBeenCalled();
  });
});
