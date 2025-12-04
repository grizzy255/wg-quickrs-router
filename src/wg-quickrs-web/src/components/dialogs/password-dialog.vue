<template>
  <div id="authentication-modal" aria-hidden="true"
       class="flex overflow-y-auto overflow-x-hidden fixed top-0 right-0 left-0 z-50 justify-center items-center w-full md:inset-0 h-[calc(100%-1rem)] max-h-full"
       tabindex="-1">
    <div class="fixed inset-0 bg-backdrop z-40"></div>
    <div class="relative p-4 w-full max-w-md max-h-full z-50">
      <!-- Modal Card -->
      <div class="relative bg-card rounded-lg shadow-sm border border-divider">
        <!-- Modal header -->
        <div class="px-6 pt-6 pb-4 border-b border-divider">
          <h3 class="text-xl font-semibold text-primary text-center">
            Sign in to wg-quickrs
          </h3>
        </div>
        <!-- Modal body -->
        <div class="px-6 py-6">
          <form action="#" class="space-y-4"
                @submit.prevent="on_submit()">
            <div>
              <label
                  :class="wrong_password ? ['text-error'] : ['text-primary']"
                  class="block mb-2 text-sm font-medium"
                  for="password">Password</label>
              <input id="password"
                     v-model="password"
                     :class="wrong_password ?
                     ['bg-badge-error-bg',  'border-input-error',  'text-error',  'placeholder-red-700', 'focus:ring-red-500', 'focus:border-input-error', 'text-error'] :
                     ['bg-input', 'border-input', 'text-primary', 'focus:ring-blue-500', 'focus:border-input-focus']"
                     class="border text-sm rounded-lg block w-full p-2.5"
                     name="password"
                     placeholder="••••••••"
                     required
                     type="password"/>
              <p v-if="wrong_password" class="mt-2 text-sm"><span class="font-medium">Oops!</span> Incorrect Password!
              </p>
            </div>

            <div class="flex justify-center">
              <div class="flex items-start">
                <div class="flex items-center h-5">
                  <checkbox :checked="remember" label="Remember me" size="4"
                            @click="remember = !remember"></checkbox>
                </div>
              </div>
            </div>
            <button
                class="w-full text-white bg-blue-600 hover:bg-blue-700 focus:ring-4 focus:outline-none focus:ring-blue-300 font-medium rounded-lg text-sm px-5 py-2.5 text-center"
                type="submit">
              Login to your account
            </button>
          </form>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import Checkbox from "@/src/components/ui/checkbox.vue";

export default {
  name: "password-dialog",
  components: {Checkbox},
  props: {
    api: {
      type: Object,
      default: null,
    }
  },
  data() {
    return {
      password: "",
      remember: true,
      wrong_password: false,
    }
  },
  methods: {
    async on_submit() {
      this.api.update_api_token(this.password).then((_) => {
        if (this.remember) {
          localStorage.setItem('token', this.api.token);
          localStorage.setItem('remember', 'true');
        } else {
          localStorage.removeItem('token');
          localStorage.setItem('remember', 'false');
        }
      }).catch((_) => {
        this.wrong_password = true;
      });
    }
  }
}
</script>

<style scoped>
</style>