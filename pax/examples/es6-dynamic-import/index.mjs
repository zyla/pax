async function compute() {
  const { square } = await import('./math');
  console.log(square(100));
}

compute();
