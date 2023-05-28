pub fn download_stable_diffusion() {
    let _install_commands = r#"
        conda create -n coreml_stable_diffusion python=3.8 -y
        conda activate coreml_stable_diffusion
        cd /path/to/cloned/ml-stable-diffusion/repository
        pip install -e .
    "#;


    let _compile_commands = r#"
        python -m python_coreml_stable_diffusion.torch2coreml
              --convert-unet 
              --convert-text-encoder 
              --convert-vae-decoder 
              --convert-safety-checker 
              -o <output-mlpackages-directory>
    "#;

    let _execute_commands = r#"
        python -m python_coreml_stable_diffusion.pipeline
              --prompt "a photo of an astronaut riding a horse on mars" 
              -i <output-mlpackages-directory> 
              -o </path/to/output/image> 
              --compute-unit ALL 
              --seed 93
    "#;
}