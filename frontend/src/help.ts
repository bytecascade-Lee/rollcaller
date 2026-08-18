import {mount} from "svelte";
import Help from "./Help.svelte";

const help = mount(Help, {
  target: document.getElementById("help") ?? document.body
});

export default help;
