import { splitProps } from 'solid-js';
import type { JSXElement } from 'solid-js';
import './Card.css';

interface CardProps {
  variant?: 'glass' | 'gradient-border';
  padding?: 'none' | 'small' | 'medium' | 'large';
  hoverable?: boolean;
  class?: string;
  style?: string;
  onClick?: (e: MouseEvent) => void;
  children?: JSXElement;
}

export function Card(props: CardProps) {
  const [local] = splitProps(props, ['variant', 'padding', 'hoverable', 'class', 'children', 'style', 'onClick']);

  const variant = () => local.variant || 'glass';
  const padding = () => local.padding || 'medium';

  const className = () => {
    const classes = ['card'];
    classes.push(`card--${variant()}`);
    classes.push(`card--padding-${padding()}`);
    if (local.hoverable) classes.push('card--hoverable');
    if (local.class) classes.push(local.class);
    return classes.join(' ');
  };

  return (
    <div
      class={className()}
      onClick={local.onClick}
      style={local.style || undefined}
    >
      {local.children}
    </div>
  );
}
