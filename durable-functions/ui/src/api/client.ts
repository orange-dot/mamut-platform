import axios from 'axios';

const API_URL = import.meta.env.VITE_API_URL || '/api';

export const apiClient = axios.create({
  baseURL: API_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// Add demo user header for audit logging
apiClient.interceptors.request.use((config) => {
  const user = sessionStorage.getItem('demo-user');
  if (user) {
    try {
      const parsed = JSON.parse(user);
      config.headers['X-Demo-User'] = parsed.email;
    } catch {
      // Ignore parse errors
    }
  }
  return config;
});

// Error handling interceptor
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    console.error('API Error:', error.response?.data || error.message);
    return Promise.reject(error);
  }
);
