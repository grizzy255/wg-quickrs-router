<template>
  <div class="my-0.5 truncate flex items-center relative ml-2">
    <!-- Label -->
    <checkbox v-if="valueField" :checked="modelValue.enabled" :label="`${label}:`" class="mr-1" size="5" :disabled="disabled"
              @click="emit_ev(!modelValue.enabled, modelValue[valueField])"></checkbox>
    <field v-else :field="`${label}:`" class="mr-1"></field>


    <!-- Input -->
    <input
        :disabled="disabled || (valueField ? !modelValue.enabled : false)"
        :class="[inputColor, getTextColorForInput(inputColor)]"
        :list="`${label}-list`"
        :placeholder="placeholder"
        :value="valueField ? modelValue[valueField] : modelValue"
        class="rounded pl-1.5 pt-[2px] pb-[2px] my-0.5 focus:outline-none focus:ring-0 border-1 border-input focus:border-input-focus outline-none w-full text-lg grow bg-input disabled:bg-button disabled:text-muted"
        type="text"
        @input="valueField ? emit_ev(modelValue.enabled, $event.target.value) : $emit('update:modelValue', $event.target.value)"/>

    <!-- Undo Button -->
    <undo-button v-if="!_fast_equal(modelValue, valuePrev) && !disabled"
                 :disabled="_fast_equal(modelValue, valuePrev)"
                 :alignment-classes="undoButtonAlignmentClasses"
                 image-classes="h-5"
                 @click="$emit('update:modelValue', valuePrev);">
    </undo-button>
  </div>

</template>

<script>
import FastEqual from "fast-deep-equal";
import UndoButton from "@/src/components/ui/buttons/undo.vue";
import Checkbox from "@/src/components/ui/checkbox.vue";
import Field from "@/src/components/ui/field.vue";

export default {
  name: "input-field",
  components: {Field, Checkbox, UndoButton},
  props: {
    modelValue: null,
    valuePrev: null,
    label: "",
    placeholder: "",
    inputColor: "",
    disabled: false,
    valueField: null,
    undoButtonAlignmentClasses: ""
  },
  emits: ['update:modelValue'],
  methods: {
    _fast_equal(s1, s2) {
      return FastEqual(s1, s2);
    },
    emit_ev(enabled, value) {
      this.$emit('update:modelValue', {enabled, [this.valueField]: value});
    },
    getTextColorForInput(inputColor) {
      // Return appropriate text color based on input background color
      if (!inputColor) return 'text-primary';
      
      // If it's a badge background, use the corresponding badge text color
      if (inputColor.includes('badge-success-bg')) {
        return 'text-badge-success-text';
      }
      if (inputColor.includes('badge-error-bg')) {
        return 'text-badge-error-text';
      }
      if (inputColor.includes('badge-warning-bg')) {
        return 'text-badge-warning-text';
      }
      if (inputColor.includes('badge-info-bg')) {
        return 'text-badge-info-text';
      }
      
      // Default to primary text color
      return 'text-primary';
    }
  },
}
</script>

<style scoped>

</style>