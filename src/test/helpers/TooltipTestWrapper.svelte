<script lang="ts">
  import Tooltip from '../../lib/components/ui/Tooltip.svelte';

  interface Props {
    text: string;
    position?: 'top' | 'bottom' | 'left' | 'right';
    delay?: number;
    buttonText?: string;
  }

  let { text, position, delay, buttonText = 'Hover me' }: Props = $props();
</script>

<Tooltip {text} {position} {delay}>
  <button type="button">{buttonText}</button>
</Tooltip>
