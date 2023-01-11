const DOMparseChildren = (children: any[]) => {
  return children.map(child => {
    if(typeof child === 'string') {
      return document.createTextNode(child);
    }
    return child;
  })
};

const nonNull = (val: any, fallback: any) => {
  return Boolean(val) ? val : fallback
};

const DOMparseNode = (element: any, properties: {[key: string]: any}, children: any): Element => {
  const el = document.createElement(element);
  Object.keys(nonNull(properties, {})).forEach(key => {
    el[key] = properties[key];
  })
  DOMparseChildren(children).forEach(child => {
    el.appendChild(child);
  });
  return el;
}

export const DOMcreateElement = (element: any, properties: {[key: string]: any}, ...children: any[]) => {
  if(typeof element === 'function') {
    return element({
      ...nonNull(properties, {}),
      children
    });
  }
  return DOMparseNode(element, properties, children);
}
