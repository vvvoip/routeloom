const select=document.querySelector('#sample');
if(select) {
 const results=JSON.parse(document.querySelector('#demo-data').textContent);
 select.addEventListener('change',()=> {
  document.querySelector('#decision').textContent=results[Number(select.value)];
 });
}
