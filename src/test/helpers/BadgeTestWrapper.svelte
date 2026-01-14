<script lang="ts">
  import Badge from '../../lib/components/ui/Badge.svelte';

  interface Props {
    variant?: 'default' | 'success' | 'warning' | 'error' | 'info' | 'muted';
    size?: 'sm' | 'md';
    glow?: boolean;
    pulse?: boolean;
    text?: string;
  }

  let { variant, size, glow, pulse, text = 'Badge Text' }: Props = $props();
</script>

<Badge {variant} {size} {glow} {pulse}>
  {text}
</Badge>
