<template>
  <div class="inline-flex items-center shrink-0">
    <label class="flex items-center cursor-pointer relative">
      <!-- Checkbox -->
      <input
          :disabled="disabled"
          :checked="checked"
          :class="checkboxSizeClass"
          type="checkbox"
          @change="$emit('click');"/>

      <!-- Checkmark Icon -->
      <span v-if="checked"
            class="absolute text-white opacity-100 top-13/24 left-0.5 transform -translate-y-1/2 pointer-events-none">
        <svg :class="iconSizeClass" viewBox="0 0 20 20" xmlns="http://www.w3.org/2000/svg">
          <path clip-rule="evenodd"
                d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                fill="currentColor" fill-rule="evenodd"></path>
        </svg>
      </span>

      <!-- Label text -->
      <field :field="label" class="ml-2"></field>
    </label>
  </div>
</template>

<script>
import Field from "@/src/components/ui/field.vue";

export default {
  name: "checkbox",
  components: {Field},
  props: {
    checked: false,
    disabled: false,
    label: "",
    size: {
      type: Number,
      default: 6
    }
  },
  emits: ['click'],
  computed: {
    checkboxSizeClass() {
      // Map size to Tailwind classes (must be static for Tailwind to work)
      const sizeMap = {
        4: 'h-4 w-4',
        5: 'h-5 w-5',
        6: 'h-6 w-6',
        7: 'h-7 w-7',
        8: 'h-8 w-8'
      };
      return sizeMap[this.size] || 'h-6 w-6';
    },
    iconSizeClass() {
      // Map size-1 to Tailwind classes
      const sizeMap = {
        4: 'h-3 w-3',
        5: 'h-4 w-4',
        6: 'h-5 w-5',
        7: 'h-6 w-6',
        8: 'h-7 w-7'
      };
      return sizeMap[this.size] || 'h-5 w-5';
    }
  }
}
</script>

<style scoped>

</style>